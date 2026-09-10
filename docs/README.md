# Documentation

Index of everything under `docs/`.

## Testing

- [**testing.md**](testing.md) — the design of holler's test harness: the process-level wire harness (real `holler hub serve` + real `holler body run` + `stub-acp` over loopback), the canary, the shared `tests/support` API, the interop opt-in, and why Windows is off the CI matrix.
- [**running-tests.md**](running-tests.md) — how to run the tests: the `cargo test` commands, the `scripts/test-run.rb` runner and its selection flags, the `Automation` grammar, and the Ruby/octokit gotchas.

## Protocols

- [**protocol/**](protocol/README.md) — the protocol surface by hop: versioned, cross-machine **wire** pages (protocol v2 is JSON-RPC 2.0 over WebSocket) and the internal channels.

## Decisions

- [**adr/**](adr/README.md) — Architecture Decision Records: one row per ADR, numbered in order, each pointing at the issue that carries it. [ADR 0002](adr/ADR-0002.md) is the one the testing docs lean on most (it retires the old two-repo decisions and records why Windows is off the CI matrix).

## Guidance

- [**orchestrating.md**](orchestrating.md) — the orchestrator pattern: dispatch with `say`, don't poll `roster`, block on a deterministic `wait`, redirect with `interrupt`, resolve with `answer` (issue #142).
- [**guidance-effective-agent-monitoring.md**](guidance-effective-agent-monitoring.md) — how to monitor a body/agent effectively (operating guidance, not a protocol or an ADR).
