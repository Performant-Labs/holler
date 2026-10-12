
## A (up-front plan review) — 2026-10-12
- **Decided:** PASS, six warns (payload for `ReaderFailed`; no flat re-export of generic names; both-streams-then-wait rule; read-error semantics per `over` mode; named homes for the OpenCode/Claude mapping; `&mut Command` and a stub using an existing `RunError` variant).
- **Assumed:** the operator's six decisions of 2026-10-12 are final and not re-reviewed; thread names are not pinned by any test (not checked).
- **Hedged:** the read-error semantics (finding 4) are unreachable in tests, so the choice is a judgement, not verified.
- **Evidence:** read probe.rs 1-330, the three exec.rs in full, ADR-0021 lines 76-92 and 385-410, lib.rs of holler-pane, `grep try_wait crates/*/src`, call sites of `exec::` in the three adapters.

## T (author / RED) — 2026-10-12
- **Decided:** RED in two forms, as the brief plans. A stub `bounded.rs` (contract types plus a `run` that answers `OutputLost`) so 18 characterisation tests fail on assertions; a 4-test source guard that fails today naming the four private copies. Applied A's warns 1, 2 and 6 to the stub's types (`ReaderFailed(io::Error)`, no flat re-export, `&mut Command`). Dropped from the brief's list a stdout-`Discard` case and a stdin-null case: no consumer for the first, and the second cannot be told apart from a null inherited stdin.
- **Assumed:** `Debug` on `RunError` is all the tests may rely on; the host and the probe will both pass `&mut Command`. Group-id behaviour is read through `ps -o pgid=` on Linux and macOS alike.
- **Hedged:** the guard's `fn wait`/`fn collect` prefix match could trip a legitimately named helper in a kept host `exec.rs`; that is intended but F may need to rename. The ADR-0021 amend-first commit is absent from the branch; flagged to O, not done by T.
- **Evidence:** `cargo test -p holler-pane --test bounded_runner_test` 0/18 on the stub, 18/18 x3 on a throwaway reference runner (discarded); `--test one_runner_guard_test` 0/4; `cargo test -p holler-pane` otherwise green; clippy `-p holler-pane --all-targets -D warnings` clean; `scripts/lint.sh` exit 0.
