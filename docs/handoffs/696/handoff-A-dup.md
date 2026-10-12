# Handoff-A-dup: #696 - one bounded subprocess runner  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** issue-696-implementation
**Diff base:** origin/main (05471b9)   **Diff head:** 1922a19
**Reuse map:** docs/handoffs/696-brief.md ("Reuse map")
**Verdict:** PASS

## Summary
F extended the object the map named. `probe.rs`'s `run_bounded` family became `holler-pane/src/bounded.rs`, and `run_probe` and all three adapters call `bounded::run`. All four private runner copies are gone. `grep` for `try_wait`, `recv_timeout`, `read_to_end`, `thread::Builder` and `.kill()` outside `bounded.rs` finds no subprocess code (the hits are herdr's socket exchange, scoped fan-out threads, the CLI import file read and load-test code, none of them a runner). The only `Command::new` sites left are command builders and the two `kill` spawns that are intentionally separate.

## Findings
| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-adapter-opencode/src/attach.rs:~250-310`, `crates/holler-adapter-claude/src/seams.rs:~77-124` | Near-identical `Ran` + `capture` + `cannot` mappings in two adapters. The error texts differ (OpenCode one_lines the full error, Claude prints `kind()` only) and the adapters may not depend on each other. This is Decision 2's intended outcome, so not a block. | Follow-up only if a third consumer appears: a `RunError` helper in `holler-pane` taking a text policy. |
| 2 | warn | `crates/holler-pane/src/bounded.rs:~335` vs `crates/holler-adapter-host/src/exec.rs:~122` | Two `kill -s ... -- -<pgid>` builders, the runner's best-effort `KILL` and the host's classifying `kill_group`. The brief puts merging them out of scope (they differ in purpose, and the host honours a fake binary). | Keep as a recorded follow-up. |
| 3 | warn | `tests/` of host, claude and opencode | The ETXTBSY-safe `script` helper (write, chmod, warm up) now exists in three test crates (`host/tests/common/mod.rs`, `claude/tests/seams_test.rs`, `opencode/tests/tmux_capture_test.rs`). Test crates cannot share it across crates today. | Optional follow-up: a shared dev-only helper. |

Drift checks: no new crate edge or dependency (adapters already depended on `holler-pane`). The adapters' `Ran`/`Failed` types are thin result mappers, not engines. `bounded.rs` is 376 lines and no touched file is near 800. No personal infrastructure names. `send_prompt`, wire/protocol and persistence are untouched. ADR-0021 was amended in the same change.

## Notes for F
None required (PASS). F's own flags: the adapters' reap is now bounded at 1 s instead of a blocking `wait`. That follows from "one engine" and from Decision 4's spirit, and I confirm it as acceptable architecture-wise, since it is the probe's existing bound.
