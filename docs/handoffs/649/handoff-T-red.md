# Handoff-T-red: Phase 4 - #649 hub wiring and the scratch integration scenario  (re-entry after the diff gate's round-1 BLOCK)

**Date:** 2026-10-11
**Branch:** issue-649-implementation (worktree `.claude/worktrees/0649-live-harness`, head `2280399` plus this phase's staged changes)
**Brief / wireframe reviewed:** `docs/handoffs/649-brief.md` (unchanged since `451e2b3`); `docs/handoffs/649/handoff-A.md` (fifth pass, W-S and W-T); `docs/handoffs/649-diff-result-r1.md` as A summarises it. No wireframe (no UI surface).

This handoff replaces the first cycle's (`e4f1d36`); git history keeps that one. Everything it lists still stands and still passes; this cycle adds to it.

## A precondition

Confirmed: A returned **PASS** on the fifth pass (`2280399`). Its notes for T-red:
- B-1 and B-2: no RED owed (W-S's evidence settles both). None authored.
- W-T: the one new behaviour of this cycle. Pinned below, as A suggested, as a row of D6's test.
- The optional rows (the 10 s lower bound, NIT-2, NV-6): all three taken, see below.

## Tests authored or changed

| Test (module::name) | Criterion / behaviour pinned | Tier, and why |
|---|---|---|
| `wired::a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server` **(new, RED)** | A's W-T under D6 and W-13. `HOLLER_TMUX_SOCKET=/tmp/hlr649-\xff/tmux.sock` (absolute, so only the encoding is wrong) is malformed, not unset. Through `Wiring::connect` and `ports()`: `connect` succeeds (D26), then `host.ensure_session`, `host.ps`, `harness.health` and `harness.shown_session` each answer `usage` naming the variable, nothing appears in the `TMUX_TMPDIR` trap, and the `opencode` trap never runs. | In-process wiring against a real hub, the same form as D6's existing row. The bug is in `connect`'s own lookup (`std::env::var(name).ok()`), which `env.rs`'s unit tests cannot see because they inject the lookup. |
| `wired::a_relative_tmux_socket_is_usage_from_the_host_and_the_harness_never_the_default_server` (refactored, passes) | D6, W-E, unchanged. Its body is now the shared `assert_the_socket_is_usage_never_the_default_server(&OsStr)`, so the two rows are one check with two values. | Same. |
| `fake_hub::a_read_and_a_write_answered_after_6_s_succeed` **(new, passes at HEAD; a pin, not a RED)** | The diff reviewer's W-1: D3, D9, I5 from below. A `pane/get` and a `pane/cas_put` the hub answers after 6 s each succeed (`Ok(None)`, and the written pane back). With the two hang rows (`< 13 s`) the store's bound is now pinned between 6 s and 13 s. | In-process client against the fake hub: the cheapest way to delay a reply on purpose (D13). |

**Test support changed (no test of its own):**
- `main.rs`: `with_env` takes any `AsRef<OsStr>` value, so a row can set a value that is not UTF-8 (A suggested `OsStr::from_bytes` under `with_env`'s lock). The existing caller is unchanged in behaviour.
- `wired.rs`: `kill_trapped_servers` runs `tmux -S <socket> kill-server` for every socket under the trap's `tmux-<uid>/` before the assertion. Without it, a wiring that reaches the default socket (as HEAD does for this row) leaves a live tmux server behind once the trap directory is removed. Checked after the RED run: the pane pid `host.ps` reported (778017) is gone, and no `hlr649` tmux process is left.
- `scratch/mod.rs`, NIT-2: `herdr_snapshot` takes its labels from `WORKSPACES` rather than repeating them.
- `scratch/mod.rs`, NV-6: `Scratch` saves the three sentinel variables before planting them and puts each back (set or removed) on drop, rather than removing them.

**Not authored:** no RED for B-1 or B-2. Both are refuted by the code and by conformance Case 1 against a real hub (A's W-S); F adds the evidence.

## RED confirmation

Command: `cargo test -p holler-cli --test pane_integration` (default run).
Result: `test result: FAILED. 60 passed; 1 failed; 4 ignored; 0 measured; 0 filtered out; finished in 119.18s`.

The one failure is the new W-T row, on its assertion, against the behaviour A described (the value read as unset, so tmux's default server in the trap):

```text
---- wired::a_tmux_socket_that_is_not_utf8_is_usage_never_the_default_server stdout ----
panicked at crates/holler-cli/tests/pane_integration/wired.rs:120:5:
HOLLER_TMUX_SOCKET="/tmp/hlr649-\xFF/tmux.sock"
host.ensure_session: want `usage` naming HOLLER_TMUX_SOCKET; got Ok(())
host.ps: want `usage` naming HOLLER_TMUX_SOCKET; got Ok([778017])
harness.health: want `usage` naming HOLLER_TMUX_SOCKET; got Ok(false)
harness.shown_session: want `usage` naming HOLLER_TMUX_SOCKET; got Err(Unavailable { what: "the Herdr socket \"/nonexistent-hlr649/herdr.sock\" cannot be connected to (entity not found)" })
touched in the trap directory: ["tmux-1000"]
```

`ensure_session` succeeding and `tmux-1000` appearing in the trap are the default-server fall-through itself: not a compile error, a missing `[[test]]` or a timeout.

**The 6 s pin discriminates.** A mutation check: with `CALL_TIMEOUT` in `wiring/store.rs` set to 5 s locally, the row fails with `the read waits past 5 s: Err(Timeout { op: "pane_store.get" })`. The file was restored with `git checkout` straight after (`git status` shows it clean).

**Other checks, at this phase's tree:**
- `cargo clippy -p holler-cli --all-targets -- -D warnings`: clean.
- `bash scripts/lint.sh`: exit 0 (only the existing size warnings in other crates). `fake_hub.rs` is now 425 lines, `wired.rs` 141.
- Opt-in, on scratch instances: `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1 scenario::the_two_grid_spellings`: 1 passed, 15 s. This exercises the NIT-2 edit (`herdr_snapshot`) and the NV-6 drop. No `/tmp/hlr649-*` directory and no `hlr649` process left afterwards.

## Notes for F

- **Code:** W-T's fix in `Wiring::connect` (`wiring.rs:104`). The row requires the value to answer `usage` naming `HOLLER_TMUX_SOCKET` from the host and the harness. A's suggested form (`var_os(name).map(|v| v.into_string().unwrap_or_default())`) does that, since `absolute` refuses the empty string. A lossy conversion would not: `/tmp/hlr649-\u{FFFD}/tmux.sock` is absolute and would be taken as a socket path.
- **Evidence:** W-S's entries in `evidence.md` (A's list). No test of this phase needs a new evidence entry: everything the W-T row relies on (`absolute`, `tmux_socket`, `connect`) is in the diff.
- No change for B-1 or B-2.

## Ready for F

**RED is valid.** One new test fails on the W-T assertion against HEAD; the rest of the suite (60 tests, the new 6 s pin included) passes and must stay green.

`T-red complete, RED is valid. F may implement against the authored tests.`
