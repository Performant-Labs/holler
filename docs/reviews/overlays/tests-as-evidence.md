# Holler overlay — tests-as-evidence

Project delta on `~/Projects/playbook/workflow/skills/tests-as-evidence/SKILL.md`.
Where this file conflicts, this file wins.

- Real hub + body processes only for anything claiming "integration" or "e2e" coverage.
  A test that only exercises `holler-proto` serde round-trips is unit evidence for the
  wire format, not for hub/body behavior.
- Golden wire files (`crates/holler-proto/tests/golden/`) are evidence for wire *shape*
  only. `BLESS=1 cargo test -p holler-proto --test golden_test` reorders every unrelated
  golden file's JSON keys alphabetically (a stale `serde_json` `preserve_order` artifact,
  not real drift) — a PR that blesses golden files must show `git diff --stat` limited to
  the files the story actually changed, or the "test evidence" is noise.
- `#[ignore]`d tests (e.g. `crates/holler-cli/tests/acp_driver_crash_test.rs`) are not
  evidence of anything until un-ignored and run in CI. A story that adds an `#[ignore]`d
  regression test without a tracked follow-up issue to un-ignore it has not proven the
  fix.
- Death modes this diff can actually hit, in order of how often they've bitten this repo:
  - A session's actor task panicking or exiting silently (no crash isolation shown across
    a real multi-session `SessionManager` run, not just a single-session unit test).
  - A liveness/heartbeat gap between hub and body — either side hanging on peer death
    without a timeout (see the hub `circuit.rs::session_loop` vs body
    `connection.rs::live_loop` asymmetry).
  - Reconnect/resume behavior after a dropped WebSocket, including whether in-flight
    `session/prompt`/`session/cancel` state survives or is correctly discarded.
  - ACP driver crash/hang on subprocess exit (the real upstream `agent-client-protocol`
    rust-sdk bug class) — a fix here needs a test that kills the child process, not just
    one that asserts on a mocked driver.
  - Golden-file wire drift silently accepted because a blessing run touched unrelated
    fixtures (see above).
- Allowed to ignore: structural nits, and speculative refactors with no behavior change.
