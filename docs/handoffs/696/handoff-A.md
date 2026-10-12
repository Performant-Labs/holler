# Handoff-A: #696 - one bounded subprocess runner  (up-front plan review)

**Date:** 2026-10-12
**Branch:** issue-696-implementation
**Brief reviewed:** docs/handoffs/696-brief.md   **Reuse map:** the brief's "Reuse map" paragraph   **Wireframe:** N/A (no UI)
**Verdict:** PASS

## Summary
The plan extends the right object: the probe's `run_bounded` family in `holler-pane`, the one crate all three adapters
already depend on, so no crate edge, no new dependency and no layering change. I re-verified the brief's file:line
claims against the worktree (the four `Command::new` sites, the only `try_wait` sites, the single call sites at
`attach.rs:217` and `seams.rs:45,67`, the `with_kill_binary` seam, the ADR-0021 section 5 sentence at line 398-399).
No block findings; six warns that F and T should resolve in the first commits, none changes the design.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | `RunError::ReaderFailed` (Decision 2) has no payload | pattern consistency / AC 3 | Today the host text is `"{program}: no thread to read its output: {e}"` and the OpenCode text is `"cannot read the output of {program}: {error}"` (Claude: the `kind()`). `{e}` is the `io::Error` of the failed thread spawn. A payload-free `ReaderFailed` cannot reproduce those texts, so "no string changes" cannot hold. | Make it `ReaderFailed(io::Error)` (the `Option` from `child.stdout.take()` returning `None` is unreachable with piped stdout; fold it into the same variant with a synthetic `io::Error`, or keep the probe's `ReaderFailed` text for it). The probe's `verdict` ignores the payload. |
| 2 | warn | Flat re-export of `Spec`, `Ran`, `Capture` from `lib.rs` (Decision 1) | naming / structure | `lib.rs` says root re-exports are the common path and the rest "stays under `error::`"; `Ran`, `Spec` and `Capture` are generic names that collide with each adapter's own `Ran` and invite import clashes. | Re-export nothing flat beyond what the probe needs, or only `bounded::run`; adapters use `holler_pane::bounded::{...}`. If something is re-exported flat, give it a specific name (`BoundedRun`, `BoundedSpec`). |
| 3 | warn | "Read stdout to the end first, wait after" (Decision 4) with a second stream | concurrency / correctness | The adapters drain stderr on a second thread. The pid-reuse rule holds only if `try_wait` is not called until BOTH streams have ended (or the deadline passed), and an `Overflow` on either stream must kill. The brief names stdout only. | Write the rule as "every kept stream to its end, then wait"; one channel for both readers; a `Discard` stream starts no reader. Add a characterisation case (stderr larger than the pipe, stdout quiet) so a one-stream engine fails. |
| 4 | warn | Read errors | behaviour preservation | The probe maps a failed stdout read to an error (`ReadFailed`); all three adapters ignore a read error ("ends the drain with what was read; the exit status decides"). The brief's `RunError` has `OutputLost` only for a lost channel, and does not say which mode wins. | Decide in the code and the doc comment: under `over: Truncate` a read error ends the capture with what was read (adapters), under `over: Fail` it is an error (probe). Map the probe's `ReadFailed` text from that error. Unreachable in practice, but the stated "no behaviour change" should be true by construction. |
| 5 | warn | Where the OpenCode/Claude mapping lives once `exec.rs` is deleted | file structure | AC 2 deletes both files, but `OpenCode Ran::reason()` (one_line), `cannot(...)`, the `timeout(op)` mapping and the lossy decode are needed at `attach.rs:217-224` and `seams.rs:45,67`. The brief names only the call-site edits. Two near-identical mappings in two adapters are the intended outcome of Decision 2, but they must not grow into a third copy. | Name the homes: a small private fn in `attach.rs` (240 lines) and in `seams.rs` (132 lines), each mapping `RunError` to its own texts. Keep the host's wrapper in `host/src/exec.rs` (225 lines, minus the deleted loops). All stay well under 800. |
| 6 | warn | `run(command: Command, ...)` by value, and the RED-2 stub `Err(RunError::Unavailable)` | API shape / test plan | The host passes `&mut Command` (`lib.rs:245`, `exec.rs:209`); by value forces edits to those callers. Decision 2 lists no `Unavailable` variant, so the stub as written does not compile against the decided type. | Take `&mut Command` (what `Command::spawn` takes) or accept the caller edits knowingly. For the stub use an existing variant (`OutputLost`). The runner, not the caller, sets stdio per `Spec` and `process_group(0)` for `Group`, on Unix only. |

Not findings, checked and consistent: the host's `kill_group` and `with_kill_binary` stay in the host (different purpose, classifies `No such process`); the probe's best-effort `kill_group` stays in the runner; no `kill`-program seam (no consumer, `dead_code` denied); `KillScope::Child` keeps adapters in the terminal's foreground group; the ADR note is amend-first and should also fix the sentence at ADR-0021 line 398-399 ("Its one direct side effect is the probe runner (`run_probe`, #663)") and the section 2 text at lines 80-86, not section 5 alone; source guard allowlist should be exactly `crates/holler-pane/src/bounded.rs` (no `try_wait(` in `probe.rs` afterwards) and must tolerate the deleted `exec.rs` files; `holler-adapter-herdr` has no subprocess runner; no other `try_wait` exists in `crates/*/src`. File sizes: `probe.rs` shrinks from 577, `bounded.rs` is new, no touched file passes 800. No persistence, wire, token or secret surface. The runner never echoes an argument, which the adapters' "name the program, none of its arguments" rule needs; keep that in the `RunError` doc comment.

## Notes for O
Not required (PASS). F and T should fold findings 1-6 into the first commits; findings 1 and 6 change the public types T stubs, so T should apply them before landing the stub.

## Patterns referenced
- `crates/holler-pane/src/probe.rs` (`run_bounded`, `kill_and_reap`, `wait_until`, the pid-reuse rule)
- `crates/holler-adapter-host/src/exec.rs`, `crates/holler-adapter-opencode/src/exec.rs`, `crates/holler-adapter-claude/src/exec.rs`
- `crates/holler-pane/src/lib.rs` (re-export convention); `docs/adr/ADR-0021.md` lines 80-86, 398-402
- `crates/holler-adapter-host/tests/fake_tmux_test.rs:761` (source-guard idiom)
