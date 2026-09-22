# Changelog

All notable changes to `holler` are documented here, per [Keep a
Changelog](https://keepachangelog.com/en/1.1.0/). See
[`docs/releasing.md`](docs/releasing.md) for the entry structure and the process that
fills this file in at release time.

## [Unreleased]

### Enhancements
- Linux arm64 (aarch64) release binary: `holler-ubuntu-arm64`, published alongside the existing
  macOS and Linux x86_64 assets on the [v0.2.0 release](https://github.com/Performant-Labs/holler/releases/tag/v0.2.0),
  built and verified on GitHub's hosted `ubuntu-24.04-arm` runner via the new
  `.github/workflows/release-arm64.yml` (`workflow_dispatch`-triggered). `install.sh` and the
  [Homebrew tap](https://github.com/Performant-Labs/homebrew-tap)'s formula both detect and
  install it automatically on Linux arm64. See [`docs/releasing.md`](docs/releasing.md)'s
  platform table.
- Homebrew install: `brew tap Performant-Labs/tap && brew install holler`, via the new
  self-hosted [Performant-Labs/homebrew-tap](https://github.com/Performant-Labs/homebrew-tap)
  (fetches the real published release binary, real sha256-verified — not a source build). A
  `scripts/update-formula.sh` there bumps it for future releases; wired into
  [`docs/release-checklist-template.md`](docs/release-checklist-template.md) as its own step so
  it can't silently fall behind a release.
- `install.sh`: a one-line installer (`curl -fsSL .../install.sh | sh`) that downloads the
  right platform binary from the latest (or a pinned, `HOLLER_VERSION=`) GitHub release —
  no `cargo build` required to just use `holler`. Documented in the README.
- Load harness scenario 2 (`--scenario session-scale`): fixes N real `holler body run`
  processes and ramps M spawn-mode `stub-acp` sessions per body — 10 → 100 → 500, so the real
  total session count is 50 → 500 → 2500 at the default N=5 — and measures real
  `session/presence` propagation latency (a real `say`-driven state change timed until the
  hub's roster observes it — the same propagation path issue #359 fixed a race in, so this is
  also a standing regression guard for that fix), `holler roster --json` read latency as
  session count grows, and hard-fails (non-zero exit) if `hub status --json`'s `sessions`
  count ever disagrees with the real configured total. First measured baseline — fan-out
  p50 10.7ms → 40.8ms and roster-read p50 6.3ms → 34.7ms from 50 to 2500 sessions, both
  scaling with session count rather than body count — is documented in
  [`docs/testing.md`](docs/testing.md) ([#369](https://github.com/Performant-Labs/holler/issues/369), [#371](https://github.com/Performant-Labs/holler/issues/371)).
- Load harness scenario 3 (`--scenario sustained-throughput`): starts a real body fleet and
  drives a **sustained** `say --queue` rate across every session concurrently for a real,
  non-trivial duration (`--sustained-secs`, default 60s — not a burst), measuring real `say`
  round-trip latency (p50/p90/p99), whether `SessionManager`'s FIFO queue
  (`QUEUE_CAP = 64`, `crates/holler-body/src/session_manager.rs`) grows unbounded under
  sustained load or drains once load eases, and hub RSS sampled repeatedly across the run
  (not just before/after) so "flat after warmup" vs. "monotonic growth" is a real series. The
  queue-depth signal reuses `holler-body`'s existing `queue_enqueue`/`queue_dequeue`/
  `queue_full` debug events (issue #197) via a new `FleetMember::start_watched` that pipes and
  relays a watched body's stderr (`crates/holler-load-test/src/fleet.rs`) — zero new
  instrumentation landed in `holler-body` itself. First measured baseline (two independent
  60s runs, `--bodies 3 --sessions-per-body 1 --rate 20`): `say` latency p50 ~1.6–1.7s once a
  real backlog exists (vs. a ~173–176ms no-queue floor), queue depth peaked at 29 (well under
  the 64 cap, never refused) and drained to exactly 0 both runs after an 8s cooldown, and hub
  RSS rose 8.0→10.5 MiB during the drive then fell and flattened at 9.3–9.4 MiB afterward — no
  post-cooldown growth in either run. No Holler defect or harness bug surfaced. Documented in
  [`docs/testing.md`](docs/testing.md) ([#369](https://github.com/Performant-Labs/holler/issues/369), [#372](https://github.com/Performant-Labs/holler/issues/372)).
- Load harness scenario 4 (`--scenario churn`): repeated real join → run → detach cycles
  (`--churn-cycles`, default 20), hard-failing the run if `hub status --json`'s `clients` or
  `sessions` doesn't return to baseline after any cycle, plus a dead-backend probe that
  attaches a body session to a real fake-OpenCode process, `SIGKILL`s it, and times how long
  the roster takes to notice. First baseline: 20/20 cycles clean, the hub back to baseline
  11.6ms (p50) after the last body dies. The dead attach backend is **never detected**: the
  roster still shows `connected`/`idle` after 240s, past its own 180s `gone` threshold,
  because the body keeps heartbeating. Also adds `FleetMember::start_attach` and collapses
  `fleet.rs`'s duplicated join/spawn logic into shared helpers. Documented in
  [`docs/testing.md`](docs/testing.md) ([#373](https://github.com/Performant-Labs/holler/issues/373)).
- Load harness scenario 4, Run #2 additions: `--teardown-modes` (rotate `graceful`/`crash`/
  `hang`/`restart` per cycle, so a wave can die without detaching, go silent under SIGSTOP, or
  crash and rejoin on its saved credential), `--parallel-teardown`, `--resident` (a body kept up
  for the whole run, taking `say` traffic while each wave is torn down), `--label-reuse-probe`,
  `--churn-secs`, `--hang-budget-secs`, and a per-cycle series of hub RSS/threads/roster rows/
  credential records. 20-minute baseline (578 cycles, 10 bodies x 10 sessions, 57,800 `say`
  calls, zero failures): cleanup correct 578/578 in every mode (p50 6.7-19.8ms); rejoining on a
  saved credential kept the same `client_id` with no ghost rows across 144 cycles; a hung body
  is dropped at the roster's `reconnect` threshold (45.0s measured at production timers); a
  resident body saw 0 disturbances in 5,780 checks. Two findings: a gracefully detached body's
  label is never freed (the credential store grew to 5,781 records, unbounded, while roster rows
  stayed bounded), and `hub token mint` does not retry under token-store lock contention (864 of
  5,781 mints needed a retry) though the hub's own redeem path does. Documented in
  [`docs/testing.md`](docs/testing.md) ([#373](https://github.com/Performant-Labs/holler/issues/373)).

## [0.2.0] - 2026-09-21

**Note:** this section had gone unpopulated since #327 (the release-process
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
