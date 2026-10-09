# Decisions: #641 host adapter (tmux sessions, process control, the launcher primitive)

## A (Phase 3, up-front plan review) — 2026-10-09T14:52:00-06:00
- **Decided:** BLOCK on docs/handoffs/641-brief.md at 69e71b5, with 3 blocks and 9 warns (see handoff-A.md). The structure is sound and has nothing in the tree to extend:
  - the dependency direction follows ADR-0021 §5;
  - the frozen `HostPort` is implemented as is, and the conformance suite is reused;
  - no new error code, no `unsafe`, and ownership is recorded inside tmux (I6).

  The plan is still not safe to test or implement as written:
  - **B-1.** An argv element that ends in `;` splits the tmux command, so the elements after it run as tmux commands.
  - **B-2.** tmux format-expands the `-c` cwd, and `#(...)` in it runs a shell command.
  - **B-3.** `kill` has no seam, so fake-tmux tests can make the adapter signal real process groups on the pipeline host.

  Each fix is an addition: an escaping rule in `tmux.rs`, `with_kill_binary`, a test-safety rule, and the ACs in Notes for O.
- **Assumed:** the `-c` format expansion behaves in other supported tmux versions as it does in 3.7c, the only version probed. The `;` splitting does not rest on the probe alone: `man tmux`, section PARSING SYNTAX, says "a trailing semicolon is also interpreted as a command separator" and gives the escape `neww 'foo\;' bar`. The `remain-on-exit` and `exit-unattached` effects in W-8 come from tmux's documented options and were not probed.
- **Hedged:** I graded B-3 a block, not a warn. The ACs as written do not require a fake test to reach `kill`, but AC 6e invites one, and the consequence is a signal to a live process on the pipeline host. The seam also gives Decision 4 default-run coverage, which it otherwise lacks, because AC 3 and AC 4 are `#[ignore]` and CI never runs them. W-1 (two `usage` refusals only the adapter has) is a warn, not a block: nothing in the conformance suite or in a consumer story depends on those inputs today.
- **Evidence:**
  - Read the brief, issue #641, epic #633, #663, #644, #649, #667, #684, #642 and #640. Read `ADR-0021.md` in full.
  - Source read: `holler-pane/src/{ports,probe,argv,lib}.rs`, `error.rs:380-510`, `holler-pane-testkit/src/{host,lib}.rs`, `conformance/host.rs`, `tests/host_conformance_test.rs`, `holler-cli/src/pane/wiring.rs`, the precedent guard tests, `scripts/lint.sh`, `clippy.toml`, `.github/workflows/ci.yml`, `CHANGELOG.md` and the agent overlays.
  - `grep` checks: no production code spawns a process, nothing depends on `holler-adapter-host`, and no personal host name appears anywhere in the tree.
  - Three tmux 3.7c probes on private `-S` servers under `/tmp/hlr-a641-*`, each killed and deleted on exit. Commands and results are under "Probe evidence" in handoff-A.md.
