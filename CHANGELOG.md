# Changelog

All notable changes to `holler` are documented here, per [Keep a
Changelog](https://keepachangelog.com/en/1.1.0/). See
[`docs/releasing.md`](docs/releasing.md) for the entry structure and the process that
fills this file in at release time.

## [Unreleased]

**Note (2026-09-21):** this section had gone unpopulated since #327 (the release-process
infrastructure itself landed) despite many real merged PRs since — `scripts/changelog-check.sh`
only enforces that the `## [Unreleased]` heading exists, not that anyone actually adds an entry
per PR. The entries below cover this session's own work (2026-09-21), verified first-hand.
**Everything merged between #327 and the start of this session is still unaccounted for** —
that gap needs real git/PR archaeology to fill in accurately, not a guess; flagging it here so
it isn't lost, not silently closing it.

### Enhancements
- Pairing SAS confirmation gate (`holler body confirm`) — a one-time, operator-facing
  short-authentication-string check, separate from the fully-automated `run` ([#351](https://github.com/Performant-Labs/holler/issues/351)).
- ACP v1-compatibility fallback: `AcpDriver::spawn` now falls back to a plain v1 connection
  when a real harness only negotiates ACP protocol v1 (as of 2026-09-21, every real
  implementation checked does) — previously every real harness (`opencode`, the Claude Code
  ACP bridge) was completely unusable ([#363](https://github.com/Performant-Labs/holler/pull/363), root cause [#362](https://github.com/Performant-Labs/holler/issues/362)).
- Load/performance harness (`holler-load-test`): a dedicated workspace binary that starts a real
  hub, opens N concurrent real circuit connections (or a fleet of real `holler body run`
  processes driving `stub-acp` sessions), drives a controlled `say`/`interrupt`/`roster` rate,
  and reports per-call latency plus the hub's own RSS/thread/CPU usage as a human table and JSON.
  Scenario 1 (connection scale) and its first measured baseline — 200 of 200 concurrent
  connections, handshake p50 9-12ms / p99 500-518ms on a 10-core macOS box — are documented in
  [`docs/testing.md`](docs/testing.md) ([#369](https://github.com/Performant-Labs/holler/issues/369), [#370](https://github.com/Performant-Labs/holler/issues/370)).

### Bug Fixes
- `hub token mint`'s printed `join_command` omitted the `wss://`/`ws://` scheme entirely,
  producing a join line `body join` itself refuses (fail-closed on a schemeless `--server`)
  ([#356](https://github.com/Performant-Labs/holler/pull/356)).
- `server_address::parse` defaulted a schemeless `wss://host` (no explicit port) to the hub's
  own loopback port (41807) instead of the standard HTTPS port (443) that a real TLS-terminating
  proxy — e.g. `tailscale serve` — actually listens on ([#360](https://github.com/Performant-Labs/holler/pull/360)).
- A real race in `holler-body`'s session manager: `finish_turn` published a transient `Idle`
  presence notice the instant a turn ended, even when a `--queue`d prompt was already about to
  start — an operator could observe the roster go idle and fire a fresh `say`, only for the body
  to correctly refuse it moments later because the queued turn had actually started. Widens
  under real CPU/scheduling pressure, which is why it only reproduced on a loaded CI runner, not
  locally ([#359](https://github.com/Performant-Labs/holler/pull/359)).
- Concurrent authentication failed against itself: the hub read its token store under a
  non-retrying `flock` on both the `circuit/authenticate` and presence paths, so bodies
  connecting at the same time refused each other with `-32002 unauthenticated` — and, because a
  refusal also counts as a failed auth, cascaded into an IP lockout that force-closed further
  connections mid-handshake. Measured by the new load harness: 3 of 50 and 7 of 200 concurrent
  connections completed the handshake before the fix; 50 of 50 and 200 of 200 after. Extends
  [#301](https://github.com/Performant-Labs/holler/issues/301)'s fix to the two sibling call sites it missed ([#370](https://github.com/Performant-Labs/holler/issues/370)).

### Known Issues
- `http_attach_driver`'s real permission/question wire shape (the JSON payload OpenCode sends
  for a pending `session/request_permission`/question, and whether it appears on `/event` at
  all) has never been independently confirmed against a real OpenCode instance — only against
  hand-authored fake-server fixtures ([#365](https://github.com/Performant-Labs/holler/issues/365)).
- Attach-mode `interrupt` confirmation can take far longer than the CLI's ~2s client-side wait
  against a real, actively-streaming backend (observed ~21s against a real OpenCode session) —
  the underlying interrupt is correct, but `holler interrupt`'s own confirmation timeout is
  tuned for something faster than real streaming backends ([#318](https://github.com/Performant-Labs/holler/issues/318)).
- `holler roster` did not detect a real dead attach-mode backend (its process killed externally)
  for several minutes, continuing to report `connected`/`idle` — no fix landed yet, tracked as
  motivation for the load-testing churn scenario ([#373](https://github.com/Performant-Labs/holler/issues/373)).
- Windows is not a supported target — the control-socket transport is Unix domain sockets end
  to end ([#378](https://github.com/Performant-Labs/holler/issues/378)).
