# Handoff-F: Phase 6 (F, implement) - #641 host adapter (tmux sessions, process control, the launcher primitive)

**Date:** 2026-10-09
**Branch:** issue-641-implementation (worktree `.claude/worktrees/0641-host-adapter`, on top of a5ea942)
**Issue:** #641

| Field | Value |
|-------|-------|
| GitHub issue | #641 (epic #633, wave 3) |
| Working branch | `issue-641-implementation` |
| Build plan phase | none (the issue is the spec); the script's Phase 6, F |
| Input documents read | `docs/handoffs/641-brief.md` (all 785 lines, as amended after round 3), `handoff-A.md` (round 4), `handoff-T-red.md`, `decisions.md`; the tests (`tests/common/mod.rs`, `tests/fake_tmux_test.rs`, `tests/real_tmux_test.rs`); `holler-pane` `ports.rs`, `argv.rs`, `error.rs`, `pane.rs`, `probe.rs`; `holler-pane-testkit` `host.rs`, `conformance/host.rs`; `holler-adapter-herdr` (crate docs, manifest); the workspace lints, `clippy.toml`, `scripts/lint.sh`, `scripts/changelog-check.sh`, `.github/workflows/ci.yml` |
| Acceptance criteria count | 14 |
| Handoff document path | `docs/handoffs/641/handoff-F.md` |

This is a Workflow run with no human gate at F, so the table is a record, not a question.

## What was done

- `crates/holler-adapter-host/src/lib.rs` (rewritten, 476 lines; T's stub replaced): the crate docs (how it talks to tmux, exact targets, what `run` starts, ownership and the stop, escaping, the error table, the five narrowings of Decision 12, the consumer facts and tmux defaults of Decision 15); `TmuxSocket` and `TmuxHost` with T's exact public signatures; `impl HostPort for TmuxHost`; the per-call deadline (`Call`); `run`'s tag and its cleanup `kill`; `stop_owned`'s two phases (`settle`).
- `crates/holler-adapter-host/src/exec.rs` (new, 225 lines): one child under a deadline (null stdin, stdout and stderr drained on two threads into a channel, `try_wait` polled every 2 ms, `Child::kill` and `wait` at the deadline, no spawn once the deadline has passed); `kill_group`, the one `kill -s <SIG> -- -<pgid>` call, with `LC_ALL=C`.
- `crates/holler-adapter-host/src/tmux.rs` (new, 319 lines): the pure argument vector of every tmux call, `escape` and `escape_cwd` (Decision 13), the strict parsers (`Pid` in `2..=i32::MAX`, `Window`, `new-window -P`, `ps`, the stop listing), `is_session_dir` (Decision 3, B-6), and `classify` (Missing / WindowGone / Duplicate / Other).
- `CHANGELOG.md`: one `## [Unreleased]` / `### Enhancements` entry for the host adapter, linking #641 and naming no host (AC 10, W-9).

## Design decisions

1. **`no such window` is a closed window, as `can't find window` is (deviation 1).** The brief's Decision 3 and Decision 8 assumed tmux answers the tag of a closed window with `can't find window`. On the first real-tmux run, `argv_and_cwd_pass_exactly` and `run_works_in_the_session_cwd` failed with `run: Unavailable { what: "tmux set-option: no such window: @1" }`: their programs exit at once, so the window is often gone before the tag. A probe on a private server (below) shows that `set-option -w -t @<id>` on a closed window prints `no such window: @<id>`, and only target lookups such as `list-panes -t @<id>` print `can't find window`. So `classify` reads both as `WindowGone`. For the tag that means `Ok`, which is Decision 3's intent. For `run`, `ps` and `stop_owned` both are "missing", which is Decision 8. Without this, every `run` of a short-lived program would answer `unavailable` and KILL the group of a pid that had already exited.
2. **A failed tag is mapped like any `run` call, after the cleanup.** For `can't find window` or `no such window` the tag is `Ok`. Any other answer first sends the cleanup `kill -s KILL -- -<pid>`, then returns an error mapped as `run` maps every call: a missing-class stderr (the server went away) is `pane-not-found`, any other stderr is `unavailable`, and a `Timeout` stays a `timeout`. The cleanup gets `max(time left, 250 ms)` and is best effort. Its own result is dropped, and the error returned is the tag's. The brief says only "then returns the error".
3. **The `what` of an `unavailable`.** A refused tmux call gives `tmux <subcommand>: <first non-empty stderr line>`, and a refused `kill` gives `kill -s <SIG>: <its first stderr line>`. When stderr is empty, the exit status's text is used. A binary that cannot be spawned gives `<binary path>: <io error>`. A refused output of tmux has a fixed text that never quotes the output (`BAD_SESSION_DIR` is the brief's exact text). The subcommand, the signal and the binary path are the adapter's own strings, so no argv element or directory can reach `what` (W-2).
4. **A drain thread can never stretch a call past its deadline.** The two drain threads send to a channel, and `collect` takes their results with `recv_timeout` against the same deadline. So a grandchild that keeps a pipe open (none does with tmux, per A's Q1 and Q2) makes the call `timeout` instead of hanging, and its thread is left detached. `exec::run` checks the deadline before it spawns, so a stop whose deadline has run out never starts another `kill`.
5. **The stop listing is read leniently, the `ps` listing strictly.** `parse_listing` skips a malformed or out-of-range line (Decision 4: "never owned"). `live` holds every line with a pid in range and `pane_dead` 0, whatever its tag, and `owned` holds the live lines whose tag equals the pid text exactly. `parse_ps` refuses the whole listing on any malformed line or pid out of range (Decision 5, W-15).
6. **A `TERM` or `KILL` that answers `No such process` is not an error.** The group emptied between the listing and the signal, and the polls then find it done. Only a failure other than `No such process` is `unavailable` (Decision 4).
7. **The window id is parsed to a `u32` and printed again** (`@{n}`). It is never passed through as tmux's text, so `@007` and an id that overflows can never reach the tag.
8. **The phase-1 grace is capped at the call's deadline.** A grace longer than the bound would leave no time for the `KILL`s; that gives a `timeout`, and the crate docs say the grace should be well under the bound. `deadline_after` caps any wait at one year, so `with_timeout(Duration::MAX)` cannot make `Instant` addition panic.
9. **The crate docs state Decision 15 with W-18's qualification (deviation 2).** The docs say "While the session's shell window exists, the session survives `stop_owned` and `ps` is not empty", and they give the relaunch order `stop_owned`, then `ensure_session`, then `run`. T left Decision 15's prose to O and S. The crate docs are F's, and they must not state the unqualified form A showed false (Q6).
10. **`ensure_session`'s `usage` message names the session, never the directory**: "cannot create the tmux session NAME: its directory must be an absolute path to an existing directory". This matches W-2's rule for `what`.

Probe (F, tmux 3.7c, a private `-S` server in a fresh `/tmp/hlr-f641-*` directory, `-f /dev/null`, `TMUX` and `TMUX_PANE` unset; an exit trap ran `kill-server` and removed the directory; no signal was sent). Output, verbatim:
```
new-session exit 0
new-window printed: [<pid> @1]          (new-window ... -- env -- true; then sleep 0.3)
## tag of the gone window @1:           set-option -w -t @1 @holler-pid 4242 \; set-option -w -t @1 remain-on-exit off
no such window: @1
exit 1
## tag of a never-made window @999:
no such window: @999
exit 1
## list-panes -t of the gone window:
can't find window: @1
exit 1
## set-option -w on a gone window, single command:
no such window: @1
exit 1
```
Afterwards no `/tmp/hlr-f641-*` directory was left.

## Reuse / extend-vs-new

As the brief's Reuse map says:
- `holler_pane::HostPort` is implemented as is.
- The `holler-adapter-host` skeleton is filled, behind T's exact public surface.
- `PaneError`, `PaneName` and `Argv` are reused, and no error variant is added.
- The `HostOp::as_str` names are reused as literals, and the test kit stays a dev-dependency.
- The conformance suite is reused untouched (T's AC 1 test drives it).
- `exec.rs` is the new object the brief justified. No production crate has a bounded runner, and `run_probe` is still #663's stub with a `ProbeResult` contract. W-7's follow-up (#663 to expose its runner from `holler-pane`) stands.
- `holler-cli/src/pane/wiring.rs` (#649) and `FakeHost` are not touched.

## Architecture notes for A

- **Layers.** `lib.rs` orchestrates the port calls. `tmux.rs` is pure, except `is_session_dir`'s one `stat`. `exec.rs` is the only process I/O. Both new modules are private, so the public surface is exactly T's stub: `TmuxSocket`, `TmuxHost::new`, the five `with_*` builders, and `impl HostPort`.
- **Dependencies.** None new in this phase. `holler-pane` is the only normal dependency; T added the dev-dependencies. There is no `libc` and no `unsafe`.
- **Contracts.** No schema, protocol, golden-file or shared-component change, and no log event (the adapter does not log).
- **Patterns followed.** `with_*(mut self) -> Self` builders with `#[must_use]`, as in `holler-hub/src/live.rs`. Crate docs in the style of `holler-adapter-herdr`. `Result` helpers throughout, with no `unwrap`, `expect` or `panic`.

## Deviations from spec / wireframe

1. `classify` adds `no such window` to Decision 8's `can't find window` (Design decision 1). It is evidenced by the probe above and by AC 11 and AC 13, which fail without it.
2. The crate docs carry Decision 15 with W-18's qualification (Design decision 9).

Wireframe: N/A (no UI surface). Nothing else deviates from the brief.

## Tier 1 self-check (incl. tests now GREEN)

All runs from the worktree, on 2026-10-09 between 18:40 and 18:58 MDT, with tmux 3.7c and procps-ng 4.0.4 `kill`.

```
$ cargo test -p holler-adapter-host
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s   (fake_tmux_test)
test result: ok. 0 passed; 0 failed; 9 ignored; ...                                              (real_tmux_test, not opted in)

$ cargo test -p holler-adapter-host --test real_tmux_test -- --ignored        (four runs, the last after the final doc edits)
run 1 (before Design decision 1): test result: FAILED. 7 passed; 2 failed   argv_and_cwd_pass_exactly, run_works_in_the_session_cwd:
                                  run: Unavailable { what: "tmux set-option: no such window: @1" }
runs 2-4 (after):                 test result: ok. 9 passed; 0 failed; 0 ignored; ... finished in 1.07s-1.20s
after every run: no /tmp/hlr-tmux-* directory, no demo-* session on the default socket, no hlr-tmux process

flake check (fake_tmux_test): 15 sequential runs and 24 runs as 8 parallel copies: 28 passed every time

$ cargo build --workspace                                   Finished `dev` profile
$ cargo clippy --workspace --all-targets -- -D warnings     Finished `dev` profile (no warning)
$ rustfmt --check --edition 2021 <the 3 src files and the 3 test files>   clean
$ bash scripts/lint.sh                                      exit 0 (crate's only warn: tests/fake_tmux_test.rs is 745 lines, T's file)
$ bash scripts/changelog-check.sh                           changelog-check: ok
$ cargo machete                                             didn't find any unused dependencies
$ grep -rnE 'pkill|killall|pgrep|pidof' crates/holler-adapter-host/src | grep -vE ':[0-9]+:\s*//'   (nothing)
$ grep -rn unsafe crates/holler-adapter-host                (nothing)
$ grep -rn 'TmuxSocket::Default\|TmuxSocket::Name' crates/holler-adapter-host/tests/real_tmux_test.rs   (nothing)

$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
127 suites: 1407 passed, 4 failed, 14 ignored
  the 4: holler-cli/tests/logging_test.rs (debug_flag_beats_env, env_none_loses_to_flag_noisy,
  banner_names_resolved_level_and_format, log_output_stays_off_stdout): "Unexpected success" because
  `holler roster` reached the live hub on this machine; see Known issues
$ HOLLER_STATE_DIR=<empty scratch dir> cargo test -p holler-cli --test logging_test
test result: ok. 11 passed; 0 failed
```

Sizes: `lib.rs` 476, `exec.rs` 225 and `tmux.rs` 319 lines, all under the 600-line warn. Clippy's `too_many_lines` (100) and `cognitive_complexity` (15) pass.

## Evidence appendix

`docs/handoffs/641/evidence.md`. It has six entries: the `HostPort` signatures, the `HostOp::as_str` names, the fake's refusal order and text, the conformance suite's readings, the pane-name grammar, and `PaneName`'s `Display`. The tmux behaviour of Design decision 1 is a probe result, not a source fact, so it is above and in `decisions.md`.

## Tests that look wrong (for T)

None is wrong. There is one coverage gap for T to decide on. `a_window_gone_before_its_tag_is_ok_and_signals_nothing` pins only the brief's `can't find window: @7`. The text tmux 3.7c actually prints for the tag of a closed window is `no such window: @7` (Design decision 1). Only the opt-in real-tmux AC 11 and AC 13 show it, so CI never does. A default-run case that answers the tag with `no such window: @7` would pin it. This is test-only. Production code already handles it.

## Known issues

- **A workspace test fails on a machine that runs a hub.** On this machine, `cargo test --workspace` has 4 failures in `holler-cli/tests/logging_test.rs`. Those tests expect `holler roster` to fail with no hub, but their helper inherits the environment, so it reached the live hub through the default state directory. With `HOLLER_STATE_DIR` set to an empty scratch directory, all 11 pass. They are unrelated to this diff: `holler-cli` does not depend on `holler-adapter-host`, and no file there changed. A CI runner has no hub.
- **The cleanup `kill` is best effort** (Design decision 2). If it fails, the untagged process of a failed tag may remain. It is documented in `tag`'s rustdoc.
- **Accepted risks from the brief, unchanged.** A directory removed between `run`'s read and `new-window` (milliseconds) still starts in `$HOME`. Pid and group-id reuse within one 50 ms poll is possible. A `new-window` that times out may leave an untagged process (W-13).

## Files changed

- `crates/holler-adapter-host/src/lib.rs`
- `crates/holler-adapter-host/src/exec.rs`
- `crates/holler-adapter-host/src/tmux.rs`
- `CHANGELOG.md`

T's files are unchanged: `Cargo.toml`, `Cargo.lock` and every file under `tests/`.
