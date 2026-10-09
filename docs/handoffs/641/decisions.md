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

## O (brief amendment after A's BLOCK) — 2026-10-09
- **Decided:** amended docs/handoffs/641-brief.md, applying A's Notes for O verbatim; no earlier decision reversed.
  - B-1, B-2: Decision 13 (one pure escape for every value the adapter did not author; `#` doubled in the cwd; tmux output validated), AC 6d and 6g extended, AC 11 (real tmux) added, a Risks line.
  - B-3: `with_kill_binary` (Decision 1), Decision 14 (no default-run test signals a pid it did not spawn), AC 6h.
  - W-1: three deliberate narrowings listed in Decision 12; the cwd is checked only when the session would be created (`has-session` on a bad cwd).
  - W-2: the `argv[0]`-contains-`=` refusal never echoes the element; `Unavailable.what` is never argv or cwd.
  - W-3: on a non-window-gone `set-option` failure, `run` KILLs the printed pid's group through the kill seam first (AC 6i).
  - W-4: AC 7's guard walks `CARGO_MANIFEST_DIR/src` at runtime, skipping `//` comments.
  - W-5: AC 8 adds `scripts/lint.sh`, `scripts/changelog-check.sh`, workspace clippy; RED is T-landed signature-only stubs answering `not-implemented`.
  - W-6: Decision 15 and two forward-compat rows (#646; #650/#654).
  - W-7: follow-up for #663 to expose its bounded runner from holler-pane (the orchestrator files it).
  - W-8: `pane_dead` 1 counts as gone; the tag invocation also sets the window's `remain-on-exit off`.
  - W-9: the personal host name in the issue title stays out of every public artifact.
- **Evidence:** O re-probed on a private `-S` server under `/tmp` (killed and deleted afterwards, no operator session touched): `-c "<dir>/q\;"` and `-c "<dir>/p##S"` give the exact session_path; `set-option ... @holler-pid N ; set-option ... remain-on-exit off` in one invocation sets both; with global `remain-on-exit on` a TERMed window lingers with `pane_dead` 1, while the window with its own `remain-on-exit off` is gone.
- **Size:** ~+255 lines (~1,455 in all), same six crate files plus two mechanical; still one run.
