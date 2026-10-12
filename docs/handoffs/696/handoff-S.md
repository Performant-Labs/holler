# Handoff-S: #696 - one bounded subprocess runner (spec audit)

**Date:** 2026-10-12
**Branch:** issue-696-implementation
**Issue:** #696
**Handoffs reviewed:** 696-brief.md, 696/decisions.md, handoff-A, handoff-T-red, handoff-F, handoff-T-green, handoff-A-dup, evidence.md; `git diff origin/main...HEAD` (27 files).

## A precondition
PASS: handoff-A (brief gate) and handoff-A-dup (diff gate) both show PASS.

## T precondition
PASS: handoff-T-green shows zero blocking issues; RED recorded in handoff-T-red (stub, 0/18 and 0/4), GREEN 18/18, 4/4 and the CI-form workspace run exit 0.

## Acceptance criteria
| # | Criterion | Proving test / evidence | Status |
|---|---|---|---|
| 1 | `pub mod bounded`; `run_probe` built on it; signature, texts, `Prober` unchanged; existing probe tests green | `bounded.rs` + `lib.rs` (+1 line); `probe.rs` `PROBE` spec + `verdict` mapping keeps every text (diff read); probe.rs inline tests untouched (18 green); `bounded_runner_test` 18 | PASS |
| 2 | Adapters call the shared runner; copies deleted; greps | `one_runner_guard_test` (4); both adapter `exec.rs` deleted and `mod exec` dropped (diff); host `exec.rs` has no `try_wait`/`recv_timeout`/`thread::Builder`/`read_to_end`/`child.kill` (diff read) | PASS |
| 3 | No behaviour change; existing tests unmodified | No existing test file in the diff (T-green); caps pinned by new `seams_test::claudes_stdout_is_cut_at_one_mib...`, `tmux_capture_test` (64 KiB), `uncapped_output_test` (host), with mutation checks; texts carried verbatim in `capture`/`cannot`/`failed` mappings | PASS (deviations below) |
| 4 | Guard + characterisation tests (timeout, kill, stderr, caps, spawn failure, past deadline) | `bounded_runner_test.rs` (18), `one_runner_guard_test.rs` (4), RED-first evidence in T-red, mutations in T-green fail the tests | PASS |
| 5 | Clippy clean, fmt on touched crates only | Clean on the four crates and on the workspace minus `holler-cli`; the full-workspace form fails only on untouched `holler-cli/tests/token_load_test.rs:60` (pre-existing, newer local clippy). Not caused by this diff. | PASS |
| 6 | `pane-control.md` line removed; stale docs updated; ADR note in scope and first commit | Line removed; probe/host/runner headers updated; ADR-0021 sections 1 and 5 amended. The "first commit" ordering is NOT met (see Spec compliance) | PASS with recorded deviation |

## Spec compliance
- Decision 1: `pub mod bounded` with `Spec{stdout,stderr,kill,poll}`, `Capture`, `Over`, `KillScope`, `Ran` (bytes), one `run`. Two deviations, both A-approved and recorded: `run` takes `&mut Command` (A warn 6), and there is no flat re-export from `lib.rs` (A warn 2; the brief said "re-exported next to `run_probe`"). Consumers use `holler_pane::bounded::`. Acceptable; it is narrower surface.
- Decision 2: typed `RunError` with the six listed variants (`ReaderFailed` carries the `io::Error`); each consumer maps to its own texts. Implemented.
- Decision 3: no kill seam, adapters `Child`, probe `Group`, host keeps `kill_group` and `with_kill_binary` (host fake_tmux 29 green). Implemented.
- Decision 4: probe's order everywhere; no signal after a `try_wait` error. Recorded in handoff-F as required. Implemented.
- Decision 5: ADR note present in section 5 (and a sentence in section 1). Amend-first ordering not achieved: T's commit precedes it. Flagged by T-red and F; unfixable without a history rewrite. Non-blocking.
- Decision 6: per-spec poll 2/5/5/10 ms. Implemented.
- Unlisted deviation (F, confirmed by A-dup): adapters' reap is now the 1 s bounded `kill_and_reap` instead of a blocking `wait`; only differs for a child surviving `KILL` over 1 s. Reasonable, follows from "one engine", untested (needs uninterruptible sleep). Noted.

## Quality audit
- Correctness: the order is read-all-kept-streams then `try_wait`; every pre-exit error kills while unreaped; `WaitFailed` never signals; a past deadline spawns nothing; a held pipe gives `Expired`. Fail-closed on a missing pipe (`ReaderFailed`). Read-error semantics per `over` are documented.
- Build guards: no `unwrap`/`expect`/`panic`/`unreachable!` in non-test code in the diff (all hits are in `tests/`); no `#[allow]` added; largest touched file `probe.rs` 420, `bounded.rs` 376, `opencode/lib.rs` 763 (untouched content), none at 900; no dead code (clippy clean).
- Protocol: no wire change; goldens and `docs/protocol/v2.md` untouched, correctly.
- Tests: real child processes, readiness by exit or pid, bounds as ranges, no fixed-sleep synchronisation beyond the ETXTBSY retry idiom. Unreachable `Overflow` mapping in the adapters has no test, by construction.
- Docs: CHANGELOG `[Unreleased]` entry links #696 and #633; `changelog-check` passed (T-green); ADR and pane-control updated; module docs state the pid-reuse rule, bound and thread-leak limit.
- Privacy: grep of the diff for personal hostnames, tailnet names, IPs, user and machine names, home paths: no hits.
- Commit hygiene: Conventional Commit subjects (`chore(#696): ...`, `docs(handoffs): ...`) with `Co-Authored-By` trailers; the pipeline commits carry no session link in the trailer (the repo hook's form). The PR does not exist yet: its AI disclosure per `CONTRIBUTING.md` is for O to add via `gh pr edit`.

## Scope check
Exactly the brief's scope. Three new adapter test files closed the brief's named gaps (T's job per the Test plan). No unrelated refactor; `cargo fmt` reflow of two untouched files was reverted. No over- or under-delivery.

## Verdict
PASS

## Advisory notes
- The ADR amend-first ordering is unmet (history already fixed); acceptable, say so in the PR body.
- Follow-ups already named by A-dup: a shared `RunError` text helper if a third consumer appears; the two `kill -s ... -- -<pgid>` builders; the triplicated ETXTBSY-safe test `script` helper.
- Pre-existing, unrelated: workspace clippy failure at `holler-cli/tests/token_load_test.rs:60` and the `interrupt_unknown_session_exit_1` liveness flake seen once by T-green.
- Rebase onto `origin/main` before merge and keep both sides in ADR-0021 and CHANGELOG (Decision 5; sibling #870 touches both).
- PR body needs the AI disclosure from `CONTRIBUTING.md`.
