# Handoff-T-red: Phase 4 - #641 host adapter (tmux sessions, process control, the launcher primitive)

**Date:** 2026-10-09
**Branch:** issue-641-implementation (worktree `.claude/worktrees/0641-host-adapter`, on top of dddd496)
**Brief / wireframe reviewed:** `docs/handoffs/641-brief.md` (as amended after round 3), `docs/handoffs/641/handoff-A.md` (round 4). Wireframe: N/A (no UI surface).

## A precondition

Confirmed: A returned **PASS** on the plan (round 4, dddd496), with three warns (W-18, W-19, W-20). Per A's Notes for O item 1, I applied all three in the tests within the brief's intent. Each is journalled in `decisions.md`.

## What T landed besides tests (the brief's Test plan)

The brief's Test plan allows T to land stub files with the exact public signatures of Decision 1 and no logic. That keeps compile errors out of the RED.
- `crates/holler-adapter-host/src/lib.rs` (stub, 123 lines) holds `TmuxSocket { Default, Name(String), Path(PathBuf) }` and `TmuxHost::new`. It also holds the builders `with_tmux_binary`, `with_kill_binary`, `with_config`, `with_timeout` and `with_stop_grace`, each `(mut self, ..) -> Self`, plus `impl HostPort for TmuxHost`. All four methods return `PaneError::NotImplemented`. **F replaces this file.**
- `crates/holler-adapter-host/Cargo.toml`: `holler-pane` (dep), `holler-pane-testkit` and `tempfile` (dev-deps), each with a consumer comment. The description no longer says "empty". `Cargo.lock` gained the new path deps.

## Tests authored

Files:
- `tests/common/mod.rs` (311 lines): the fakes and the one fake-side host helper.
- `tests/fake_tmux_test.rs` (745 lines): 28 tests, default run, no tmux needed.
- `tests/real_tmux_test.rs` (452 lines): 9 tests, all `#[ignore]`. Each returns early with `skipped: tmux not found` when `tmux -V` cannot run.

**The fakes (`common/mod.rs`).**
- Both are `/bin/sh` scripts in one private tempdir `D`. The fake tmux records every call's arguments, one argument per line, then `-=END=-`. The fake `kill` records the same way, plus a flat line (`kill.flat`) and `${LC_ALL-unset}` (`kill.env`).
- Three answer modes (W-19): `answer` gives one answer for every call; `queue` answers call N, with the last answer repeating; a body can also key on `"$*"` and on the kill record (`killed '<line>'`). No test sizes a queue by how many polls fit in a grace.
- `SAY_DIR` answers `run`'s read with `$D`, an existing absolute directory.
- Each script is warmed up once, unrecorded, before use. This retries `ETXTBSY` from a parallel test's fork.
- No fake ever signals a process.
- `host_on(fake, socket)` is the **only** fake-side `TmuxHost::new` (W-20). It always sets both fake binaries, a 5 s bound and a 200 ms grace.

**Default run (`fake_tmux_test.rs`).** Unit tier is impossible here: the adapter's behaviour *is* its subprocess vectors and its reading of their output. A fake binary is the cheapest tier that sees the real argv and environment.

| Test | Pins (AC / Decision) |
|---|---|
| `every_method_is_bounded_and_reaps_its_child` | 6a: each method on a hung fake (`exec sleep 30`), with a 200 ms bound, returns `Timeout { op }`, and `op` equals `HostOp::as_str`. The method returns in under 1.2 s. The hung pid is reaped, checked with `kill -s 0` through the `kill` binary and not `/proc` (W-19). |
| `a_missing_tmux_binary_is_unavailable_from_every_method` | 6b: all four methods return `Unavailable`, `stop_owned` included. |
| `a_missing_session_is_pane_not_found_for_run_and_ps_and_ok_for_stop` | 6c and Decision 8: five strings (the four in the AC, plus `Connection refused` from Decision 8). `ps` and `run` return `PaneNotFound`, `stop_owned` returns `Ok`, and nothing is signalled. |
| `other_tmux_failures_are_unavailable_with_its_line_and_never_the_argv_or_cwd` | 6c and W-2: three stderrs (`Permission denied`, `File name too long`, a multi-line other) give `Unavailable` from ps, run, stop_owned and ensure_session. `what` holds the first stderr line, never the argv or cwd sentinel. |
| `run_passes_the_argv_exactly_in_three_tmux_calls` | 6d, B-2, B-5, B-6, K2: exactly three spawns, each vector exact. (1) `-S <sock> list-panes -t =demo-c1r1: -F #{session_path}`; (2) the full `new-window ... -c #{session_path} -t =demo-c1r1: -- env -- prog "a b" "$(id);x" -t`; (3) the tag, `set-option ... @holler-pid 4242 ; set-option ... remain-on-exit off`. |
| `run_prefixes_a_one_element_argv_with_env` | 6d: `["prog"]` is recorded as `-- env -- prog`. |
| `run_escapes_a_trailing_semicolon_in_every_element` | 6d, B-1, Decision 13: the elements are recorded as `x\;`, `y\\;` and `\;`. |
| `every_target_names_the_session_exactly` | 6e, B-4, W-16, Decision 10. Covered paths: both ensure paths, run's read and new-window, ps, and stop_owned's listing plus at least one poll. `has-session` gets `=demo-c1r1`. Every other session target gets `=demo-c1r1:`. `new-session` gets `-s demo-c1r1` and no `-t`. No bare name appears. |
| `the_socket_and_config_flags_come_before_the_subcommand` | 6f: `Path` gives `-S p`, `Name` gives `-L n`, `Default` gives nothing, and a config adds `-f`. All of these come before the subcommand. |
| `tmux_gets_no_tmux_variables_and_nothing_added` (+ child `env_probe_child`) | 6f and W-12: the test re-runs its own binary with `TMUX` and `TMUX_PANE` set and `LC_ALL` removed. The fake must see `unset unset unset`. No `set_var`, no `unsafe`. |
| `ensure_session_with_a_cwd_it_cannot_create_in_only_asks_has_session` | 6g, Decision 6, B-6: a missing absolute dir and `.` each give exactly one call, `has-session -t =demo-c1r1`. A missing session gives `Usage`; an existing one gives `Ok`. `new-session` is never called. |
| `ensure_session_escapes_the_cwd_for_tmux` | 6g, Decision 13: `p#S` is recorded as `-c .../p##S` and `q;` as `-c .../q\;`, in one `new-session -d -s demo-c1r1` call. |
| `run_refuses_an_assignment_as_argv0_without_echoing_it` | 6g, Decision 7, W-2: `Usage` names `argv[0]` and echoes neither half of `HLR_SENTINEL_641=s3cr3t`. Only the read is called. A missing session returns `PaneNotFound` first. |
| `run_refuses_a_session_directory_that_is_relative_missing_or_empty` | 6g, B-6: a relative, missing or empty read gives `Unavailable` with neither path in `what`. Only the read is called. |
| `run_checks_the_session_before_the_argv` | AC 1's order, Decision 7: a missing session with an empty argv gives `PaneNotFound`. An existing session with an empty argv gives `Usage`, after one call. |
| `stop_owned_terms_only_owned_live_groups_then_kills_a_live_survivor` | 6h, Decision 4, W-11, W-15, W-12: five panes. The only TERM is `-s TERM -- -101`, a KILL follows the grace, and 202, 303, 404 and 4294967295 are never named. Every `LC_ALL` is `C`. |
| `a_tagged_pane_listed_dead_counts_as_gone` | 6h, W-8: `Ok`, with no KILL. |
| `a_group_that_outlives_its_pane_is_killed_after_the_grace` | 6h, W-14: exactly one `KILL -- -101`, then a `-s 0` probe after it before `Ok`. |
| `a_session_that_ends_on_term_still_has_its_group_checked` | **W-18** (A's suggested AC 6h bullet): a poll that answers `can't find session` still needs the group probe. Exactly one KILL, then a probe, then `Ok`. |
| `a_kill_failure_other_than_no_such_process_is_unavailable` | 6h: a failed TERM gives `Unavailable`, and so does a failed group probe. |
| `one_grace_is_shared_by_every_owned_group` | 6h, W-10: both TERMs come before any KILL, with one KILL per group, in under grace + 1 s. |
| `a_window_gone_before_its_tag_is_ok_and_signals_nothing` | 6i, Decision 3. |
| `a_failed_tag_kills_the_untagged_process` | 6i, W-3: `Unavailable`, and the kill record is exactly `["-s KILL -- -4242"]`. |
| `a_malformed_or_out_of_range_new_window_answer_is_unavailable_and_goes_no_further` | 6i, W-15: seven answers, each `Unavailable` after exactly two calls, with no signal. |
| `ps_lists_the_live_panes_and_refuses_an_out_of_range_pid` | Decision 5, 6i and W-15: only the live panes `{101, 202}` are listed. A `4294967295` pid gives `Unavailable`. |
| `no_broad_kill_in_source` | AC 7: walks `src/` at runtime and skips `//` text. |
| `hosts_are_built_by_one_helper_and_real_tmux_tests_use_private_sockets` | AC 9 scoped per **W-20**: `TmuxHost::new` appears exactly once on the fake side and once in `real_tmux_test.rs`. `real_tmux_test.rs` names neither `TmuxSocket::Default` nor `TmuxSocket::Name`. |

**Real tmux (`real_tmux_test.rs`, `#[ignore]`).** The process-level tier is the only one that proves tmux's own parsing, cwd and signal behaviour. Every host comes from `private()` (AC 9). That helper:
- makes a `tempfile` dir with prefix `hlr-tmux-` in `/tmp`;
- uses the socket `<dir>/s`, asserted under 100 bytes;
- writes a config with `set -g default-shell /bin/sh`;
- returns a guard whose `Drop` runs `kill-server`, then removes the dir.

The real `kill` is only used on processes the test started there.

| Test | Pins |
|---|---|
| `the_tmux_host_passes_the_host_conformance_suite` | AC 1 (`run_host_conformance(private)`, all nine cases) |
| `ensure_session_twice_makes_one_session` | AC 2: one session; `session_path` keeps the first cwd; a third call with a nonexistent cwd returns `Ok` |
| `stop_owned_kills_only_the_owned_process` | AC 3: the owned pid is gone; the shell pane, the session, the other session's pid and an outside `sleep` all survive |
| `stop_owned_escalates_to_kill` | AC 4: a TERM-ignoring group; then the W-14 member that outlives its leader |
| `missing_session_is_typed` | AC 5: no server ever started |
| `argv_and_cwd_pass_exactly` | AC 11: six hostile elements arrive verbatim; `p#S` and `#(touch ...)` cwds are exact; `pwd -P` is checked; no `ran` file appears |
| `targets_are_exact` | AC 12: the prefix case, and a window named like the session in the newest session |
| `run_works_in_the_session_cwd` | AC 13 |
| `run_refuses_a_missing_or_relative_directory` | AC 14 |

## RED confirmation

Run: `cargo test -p holler-adapter-host` (default) at 2026-10-09 18:30 MDT. Every one of the 25 behaviour tests fails on the adapter's answer, `NotImplemented`. Representative failure lines, verbatim:

```
test result: FAILED. 3 passed; 25 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
every_method_is_bounded_and_reaps_its_child ... host.ensure_session: expected Timeout, got Err(NotImplemented)
a_missing_tmux_binary_is_unavailable_from_every_method ... host.ensure_session: expected Unavailable, got Err(NotImplemented)
a_missing_session_is_pane_not_found_for_run_and_ps_and_ok_for_stop ... ps, "can't find session: x": Err(NotImplemented)
other_tmux_failures_are_unavailable_with_its_line_and_never_the_argv_or_cwd ... ps, "error connecting to /s (Permission denied)": expected Unavailable, got Err(NotImplemented)
run_passes_the_argv_exactly_in_three_tmux_calls ... left: Err(NotImplemented) right: Ok(())
every_target_names_the_session_exactly ... assertion `left == right` failed: the new-session path  left: Err(NotImplemented)
the_socket_and_config_flags_come_before_the_subcommand ... left: Err(NotImplemented) right: Ok([101])
tmux_gets_no_tmux_variables_and_nothing_added ... assertion `left == right` failed: the tmux subprocess's environment  left: "" right: "unset unset unset\n"
ensure_session_with_a_cwd_it_cannot_create_in_only_asks_has_session ... ".../hlr-missing-641": Err(NotImplemented)
run_refuses_an_assignment_as_argv0_without_echoing_it ... expected Usage, got Err(NotImplemented)
run_refuses_a_session_directory_that_is_relative_missing_or_empty ... printf 'hlr-rel-641\n': expected Unavailable, got Err(NotImplemented)
stop_owned_terms_only_owned_live_groups_then_kills_a_live_survivor ... left: Err(NotImplemented) right: Ok(())
a_session_that_ends_on_term_still_has_its_group_checked ... left: Err(NotImplemented) right: Ok(())
a_failed_tag_kills_the_untagged_process ... the tag failed: expected Unavailable, got Err(NotImplemented)
a_malformed_or_out_of_range_new_window_answer_is_unavailable_and_goes_no_further ... abc @7: expected Unavailable, got Err(NotImplemented)
ps_lists_the_live_panes_and_refuses_an_out_of_range_pid ... called `Result::unwrap()` on an `Err` value: NotImplemented
```

Every other failing test (`run_prefixes...`, `run_escapes...`, `ensure_session_escapes...`, `run_checks...`, `a_tagged_pane...`, `a_group_that_outlives...`, `a_kill_failure...`, `one_grace...`, `a_window_gone...`) fails the same way: `left: Err(NotImplemented)`, or `expected X, got Err(NotImplemented)`.

**Three tests pass at RED, by design.** They are not behaviour tests:
- `no_broad_kill_in_source` and `hosts_are_built_by_one_helper_and_real_tmux_tests_use_private_sockets` are absence guards. AC 7 and AC 9 say a thing must *not* appear, so the guards cannot be red before the code exists. They become meaningful when F writes `src/`.
- `env_probe_child` is the re-exec helper. It returns at once without its marker.

Run: `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored`, with tmux 3.7c installed:

```
test result: FAILED. 0 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
stop_owned_escalates_to_kill / argv_and_cwd_pass_exactly / ensure_session_twice_makes_one_session / run_refuses_a_missing_or_relative_directory / run_works_in_the_session_cwd / stop_owned_kills_only_the_owned_process / targets_are_exact ... ensure_session: NotImplemented
missing_session_is_typed ... ps: Err(NotImplemented)
the_tmux_host_passes_the_host_conformance_suite ... left: Err([CaseFailure { case: "ps-of-missing-session-is-pane-not-found", detail: "... expected `pane-not-found`, got `not-implemented` ..." }, ... all nine cases ...])
```

Afterwards no `/tmp/hlr-tmux-*` dir was left.

**The fakes were checked on their own** with a throwaway test, deleted afterwards and never staged. It drove both scripts by hand:
- The record keeps `a b`, `x\;` and an empty element as separate arguments.
- `queue` repeats its last answer.
- `SAY_DIR` prints `$D`.
- The listing drops `101` once `-s KILL -- -101` is recorded.
- The `kill` default answers `/usr/bin/kill: (-101): No such process` (exit 1) for `-s 0`.
- `kill.env` records `unset` when no `LC_ALL` is given.

So when F's adapter runs against them, the fakes answer as each test assumes.

**Tier-1 hygiene of the test files at RED:**
- `cargo clippy -p holler-adapter-host --all-targets -- -D warnings`: clean.
- `rustfmt --check --edition 2021` on all four files: clean.
- `bash scripts/lint.sh`: exit 0, with one warn that `fake_tmux_test.rs` is 745 lines. That is under 900.
- `cargo machete`: clean.
- Every `#[allow]`/`#![allow]` carries `// #641`.

## Notes for F

- The tests pin the vectors exactly as Decisions 3 to 10 and 13 give them. Three of those are worth stating:
  - `-S <sock>` comes first for a `Path` socket.
  - `run`'s read is `list-panes -t =NAME: -F #{session_path}`, with no `-s`.
  - The tag is one `set-option ... ; set-option ...` spawn.
- The `kill` lines are pinned as `-s <SIG> -- -<pgid>`.
- **W-18** is pinned by `a_session_that_ends_on_term_still_has_its_group_checked`. A poll that answers "missing" after the TERM counts every pane as gone, but each group is done only after `kill -s 0` reports `No such process`. Decision 8's `Ok` applies to the first listing only.
- `stop_owned` must ask `-s 0` only after the pane is gone, and it must send exactly one `KILL` per surviving group.
- `Unavailable.what` for a stderr failure must hold tmux's first stderr line.

## Ready for F

Confirmed: the RED is valid. All 25 default-run behaviour tests and all 9 real-tmux tests fail on the adapter's `NotImplemented` answer. None fails on a compile error, a harness error or a timeout. F may implement against these tests.

T-red complete, RED is valid. F may implement against the authored tests.
