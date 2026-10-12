# Decision journal: #850 (a body driver for a Claude Code pane)

Run record for the build run. The brief is `docs/handoffs/850-brief.md`.

## A (Phase 3, up-front plan review) — 2026-10-12
- **Decided:**
  - PASS, with 0 blocks and 6 warns (`handoff-A.md`).
  - Kept the preparer's design: a `ClaudeAttachDriver` over the `HarnessPort` that `holler-cli` injects, the `holler-body -> holler-pane` edge (A1), the drive rule extended to a single process, and no hub, wire or golden change.
  - Asked for: a stated choice on whether the support probe honours `HOLLER_CLAUDE_BIN` (W-1); a source guard for the one body-side `send_prompt` caller (W-2); the poll's cost and the abandoned-thread behaviour in D3 (W-3); a stale-row note (W-4); the refusal window of the two drive writes (W-5); the ADR 0014 "from the registry" wording (W-6).
- **Assumed:**
  - The brief's E-excerpts match `main` at `ef881fc` (spot-checked: the driver enum, the task's `rebind`, `driver_of`, the adapter, `claude_bin`, the hold guard).
  - `claude agents --json` costs about 90 ms a run, as the adapter's doc says.
- **Hedged:**
  - W-1: which of the two probe choices is right is the operator's; the hub uses `harnesses_confirmed` only for status and caps, so the harm is a misleading report, not a refusal.
  - W-3: the cost per tick is counted from the adapter's code, not measured.
- **Evidence:**
  - Read in full: `docs/handoffs/850-brief.md` lines 1-365, the driver enum, ADR 0014, ADR 0021 section 5, `tx_switch.rs` (switch, drive, relaunched), `tx_switch/plan.rs`, `driven.rs` (`driver_of`), `hold_single_path_test.rs`.
  - Read in part: `holler-adapter-claude/src/lib.rs`, `holler-pane/src/ports.rs`, `holler-body/src/{config,query}.rs`, `session_manager/task.rs`, `holler-cli/src/pane/wiring/env.rs`, `holler-hub` for harness uses.
  - Commands: `grep -rn '"claude"' crates/*/src`; `cat crates/*/Cargo.toml` for the crate graph; `wc -l` on the touched files.
