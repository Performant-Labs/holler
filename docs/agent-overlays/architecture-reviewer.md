## This stack

What to check in a Holler plan or diff, beyond the shared dimensions:
- **Choke points.** Prompts reach a body only through `send_prompt` (`crates/holler-hub/src/circuit/dispatch.rs`). New control-plane behavior (holds, limits, refusals) belongs there. Flag any design that enforces it at call sites or in only one delivery variant (`say`, `say --queue`, `say --replace`).
- **Wire and protocol.** The error-code table in `holler-proto` is closed and tested for uniqueness and range. New fields must be additive and optional (no version bump, ADR 0014's precedent), with golden files and `docs/protocol/v2.md` updated. An unchanged golden file changing is a defect.
- **ADRs.** A decision that contradicts or extends an ADR in `docs/adr/` needs the ADR updated in the same change, not silently ignored.
- **State and persistence.** The hub does not persist its roster; anything new that persists needs a stated location under the state dir, atomic writes, the right file permissions, and defined behavior for a corrupt or unwritable file (fail closed).
- **Concurrency.** The hub's live paths are per-connection tokio tasks that all contend for the token store lock. Blocking I/O belongs in `spawn_blocking`; a design that adds a write per heartbeat or per frame needs a throttle and a justification.
- **Identity and secrets.** Authentication is a token plus the Noise handshake, never a network address. No secret in any log line. Keys and credentials never appear in test output.
- **Size and structure.** `scripts/lint.sh` fails files at 900 lines; flag any touched file above about 800 and require the plan to name where new code goes.
- **Public repository.** No personal infrastructure names in code, docs, tests or commit metadata.

**Phase 7 (anti-duplication) candidates to check first:** the token store operations in `token.rs`, `Lockout` and `Roster` in the hub, the `log(Severity, ...)` helper, and the test harness helpers (`Hub`, `Body`, `mint_token`, `join`, `wait_for`, `StateDir`). A new near-copy of any of these is a rejection.

## Project-specific references

- `docs/adr/`, `docs/protocol/v2.md`, `docs/testing.md`
