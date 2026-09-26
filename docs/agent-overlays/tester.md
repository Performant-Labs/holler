## This stack

Rust workspace (`holler-proto`, `holler-hub`, `holler-body`, `holler-cli`, `holler-load-test`). The runner is `cargo test`; integration tests live in `crates/*/tests/`. **`holler-cli` sets `autotests = false`: every new test file must be declared as a `[[test]]` in `crates/holler-cli/Cargo.toml`, or it silently never builds.** A RED run that says "no test target named X" is this mistake, not a real RED.

The harness is `crates/holler-cli/tests/support/`: real subprocesses of the real `holler` binary (`Hub::start`, `Body::start`, `StateDir`, `mint_token`, `join`, `Hub::log_text()`), with `stub-acp` as the agent. There are no mocks of the circuit. Readiness is always observed with `wait_for` on a log line, roster row or file, never a fixed sleep. Pick the cheapest sufficient layer: unit test in the module (`#[cfg(test)]`, using the injected clock where a `Clock` exists) > codec and golden-file tests in `holler-proto` > a `holler-hub` test on a real state dir and store > a `holler-cli` process-level test with a real hub and body.

**Tier 1 commands (reproduce what CI runs).** `.github/workflows/ci.yml` and `scripts/lint.sh` are the source of truth; if they differ from this list, they win. At the time of writing:
`bash scripts/lint.sh`, `bash scripts/changelog-check.sh`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo test -p holler-cli --test docs_cli_test`, `cargo machete`, and the canary `cargo test -p holler-cli --test wire_selftest`.

**Tier 2 checks specific to this stack:**
- RED means the new test fails on an assertion about the missing behavior, not on a compile error, a missing `[[test]]` entry, or a harness timeout you cannot explain.
- Timing races are asserted as invariants ("delivered before the change, or refused, never both"), not as durations, and concurrency tests are repeated enough to surface flakes. This repo has a history of fixed-sleep flakes.
- A protocol-visible change needs golden files and `docs/protocol/v2.md` updated. Blessing goldens reorders every file, so `scripts/golden-diff-summary.sh` must show only the intended drift.
- Every documented `holler ...` command must parse (`docs_cli_test`); a bare command fragment in backticks fails it.
- Test files and support modules stay under 900 lines (`lint.sh` fails at 900). Every `#[allow(...)]` carries a trailing `// #NNN` issue link.
- Logs and errors asserted in tests must never contain a secret; assert the secret is absent.
- There is no browser, Playwright or visual surface in this repo. Do not run or request any.

## Project-specific references

- `docs/testing.md`: the harness API, the no-blind-sleeps rule, the test catalog conventions
- `crates/holler-cli/tests/support/mod.rs`, `support/onboard.rs`, `support/cmds.rs`
- `crates/holler-hub/tests/token_store_test.rs`: the pattern for lock-contention regression tests
- `crates/holler-cli/tests/auth_rejection_log_test.rs`: the pattern for asserting on the hub's log
