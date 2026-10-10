The decision journal of #640 part 3 of 3. It replaces part 2's journal, as the brief's Handoffs line says; part 2's is
in git at `0ad2d8a:docs/handoffs/640/decisions.md`.

## A (Phase 3, up-front plan review) — 2026-10-09T20:29:58-06:00
- **Decided:** PASS, with 0 block and 6 warn findings (`docs/handoffs/640/handoff-A.md`). The plan extends the objects
  its Reuse map names: `protocol::excerpt` made `pub(crate)` (one copy), `Tapped::Fail` on the one interceptor,
  `run_herdr_conformance` run rather than copied, and `Timeout.op` renamed once at the port boundary with the transport
  untouched. That is the OpenCode adapter's `Call` shape. It adds no dependency and puts every rule it sets into ADR-0021
  in the same change. Warns:
  - (1) `origin/main` moved to `e327569` (#641, 8:06 PM MDT) after the brief's baseline `dc300ab`.
  - (2) #641's merged `real_tmux_test.rs` uses plain `#[ignore]` and skips when the tool is missing. This plan adds
    `HOLLER_HERDR_SCRATCH=1` and fails when the tool is missing. The divergence is reasoned but not named in the brief or
    in `docs/testing.md`.
  - (3) The Reuse map does not name the helpers the harness re-creates: `holler-cli/tests/support` `wait_for`, `Hub` and
    `StateDir`, the host and OpenCode `exec::run`, and #696. None is reachable from this crate. The harness should hold
    one bounded runner and one poll helper.
  - (4) D4 and A4 state the `op` rule with no room for AC 16's `herdr.connect`. A4's example `harness.health` is never
    returned by the real adapter, and §2 is the better home for the rule.
  - (5) AC 19's grep also matches `adapter.rs:338`, a returned label, not a message.
  - (6) Three things stay stale: the `snapshot` doc versus A9, A7's new PROPOSED item missing from the Deferred list, and
    the test kit's `ASSUMPTION (#640)` comments, whose follow-up is unfiled.
- **Assumed:**
  - The Reuse map embedded in the brief is the survey, since this run has no `survey.md`.
  - On the automated path the run continues to T on PASS without a brief amendment, so the warns are advisory.
  - I started a fresh journal because the brief says part 3's handoffs overwrite part 2's and the conventions keep one
    journal per run.
  - #641's test, merged after the brief was written, is the one sibling opt-in convention on `main`. No other merged
    real-instance test exists.
- **Hedged:**
  - Finding 2 sits between warn and block. One merged instance is not a dominant pattern, and Decision 6 gives a written
    reason to differ that fits Herdr's risk (a socket resolved from `HOME`, `XDG_*` and `HERDR_*`), so I kept it at warn.
  - Finding 3 would become a Phase 7 rejection under this repo's overlay ("a new near-copy ... is a rejection") if its
    justification were not written. This handoff now holds that justification, which assumes Phase 7 reads it.
  - Finding 4 ships slightly wrong contract and ADR text unless the brief is reopened. F may not change D4's meaning
    or touch other ADR lines.
- **Evidence:**
  - Read: the brief in full, the three brief-gate results (r1 and r2 BLOCK, r3 PASS), and part 2's `handoff-A.md`,
    `handoff-A-dup.md` and `decisions.md`.
  - Read in full: `crates/holler-adapter-herdr/src/{adapter,protocol,transport,lib}.rs`, `tests/wire_herdr/{mod,serve}.rs`,
    `tests/{adapter,adapter_conformance}_test.rs`, `tests/common/mod.rs` and `Cargo.toml`;
    `crates/holler-pane-testkit/src/herdr.rs` and `conformance/herdr.rs`; `crates/holler-adapter-opencode/src/{lib,exec}.rs`
    and `tests/support/stub.rs`.
  - Read in part: `crates/holler-pane/src/{ports,error,pane,reconcile,probe}.rs`, `reconcile/observe.rs:155-185`,
    `holler-cli/tests/support/mod.rs:1-260`, and `holler-adapter-opencode/tests/hermetic_test.rs:1-140`;
    `docs/adr/ADR-0021.md` §1-§2, §5, §9-§12 and Consequences through Decisions taken; `docs/testing.md` from "Beyond
    loopback" on; `CHANGELOG.md`; `scripts/lint.sh`, `clippy.toml` and the workspace lints.
  - Other branches and issues: `origin/main` at `e327569`, namely `crates/holler-adapter-host/tests/real_tmux_test.rs`,
    `src/exec.rs` and `src/lib.rs:134-137`; `origin/issue-644-implementation:docs/handoffs/644-brief.md`; and
    `gh issue view 633, 641, 649, 663, 667, 696`.
  - Greps: E-5's and E-12's baselines, AC 8, 9, 19, 21, 24 and 25 at the brief gate, `Timeout {` constructions, uses of
    `op` (`detail()`, `Display`), `#[ignore` across `crates/`, `HOLLER_*` variable names, `Command::new`, `try_wait` and
    `wait_for` sites, and matches on `Tapped`.
