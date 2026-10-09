# Handoff-A: Phase 3 - #641 host adapter (tmux sessions, process control, the launcher primitive)  (up-front plan review, round 3)

**Date:** 2026-10-09
**Branch:** issue-641-implementation (at 95e2260)
**Brief reviewed:** docs/handoffs/641-brief.md, as amended after round 2   **Reuse map:** docs/handoffs/641-brief.md §Files "Reuse map"   **Wireframe:** N/A (no UI surface)
**Earlier rounds:**
- Round 1: BLOCK at 69e71b5, with 3 blocks and 9 warns. That handoff is this file as of commit 475d6fe.
- Round 2: BLOCK at 81b0ddd, with 2 blocks and 4 warns. That handoff is this file as of commit 770c948.

Every finding of both rounds is applied in the brief as amended.
**Verdict:** BLOCK

## Summary

BLOCK, on one new finding. B-5's fix is right but incomplete. `run` starts its program with `-c '#{session_path}'`, and that sends the program to the session's directory only while the directory is an absolute path that still exists.

- **Relative path.** tmux keeps a relative `-c` verbatim. Each later `run` then resolves it against the cwd of the CLI that calls it.
- **Missing directory.** If the directory is missing, tmux starts the program in `$HOME`.

In both cases `new-window` exits 0 and `run` returns `Ok`. So a coding agent can be launched, with no error, into another project or into the operator's home directory. That happens after a `--project .` or a removed worktree. No AC or suite case would catch it. The fix is two additive checks, one in `ensure_session` and one in `run`, each a closed `usage` or `unavailable` refusal, and no decision is reversed.

Four warns follow:
- **W-14.** A process-group member that ignores TERM outlives `stop_owned` once its leader exits.
- **W-15.** A pid read from tmux should be range-checked before it reaches a `kill` argument.
- **W-16.** AC 6e's wording forbids the correct `new-session -s` vector.
- **W-17.** The fallback test-helper path departs from the neighbor's convention.

The rest of the plan is consistent with existing patterns, and the round-2 fixes are applied as asked.

This is the third Phase 3 BLOCK. The role doc says that after more than two blocks, O escalates to the operator.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| B-6 | block | Decision 6 (`new-session -c <cwd>` for any existing directory, a relative one included); Decision 3 (`-c '#{session_path}'`, unchecked); Decision 15 ("a later `run` still works in A"); AC 11 and AC 13 (absolute, existing paths only) | contract fidelity (the port: the session works "in `cwd`"; ADR-0021: `--project DIR` maps to `host.cwd`, the "project directory or worktree"); validation at the boundary, failing closed | B-5's fix holds only while the session's directory is absolute and exists. (a) **Relative path.** tmux keeps a relative `-c` verbatim. `new-session -c rel` gave `session_path=rel`. A later `run` from another client cwd started its program in that cwd's `rel`, or in `$HOME` when that cwd had no `rel` (PR1; `-c .` behaves the same). Decision 6's directory check resolves the path against the CLI's cwd at that moment, so it passes. `--project` is a plain `Option<String>` (`holler-cli/src/pane/args.rs:44`) and `host.cwd` a `String` (`holler-pane/src/pane.rs:111`), and nothing canonicalizes either. (b) **Removed directory.** When the session's directory is removed after the session was made, as happens to a removed worktree, tmux falls back to `$HOME` and `new-window` exits 0 (PR5). In both cases `run` returns `Ok`, and the harness, a coding agent, works in another project or in `$HOME`. Decision 15's "a relaunch after `host.cwd` changes runs in the old directory" becomes `$HOME` once that directory is gone. No port call can read or end the session, so a consumer cannot guard against it. No AC or suite case uses a relative or a removed directory. | (a) Decision 6: the `new-session` path needs an **absolute** path to an existing directory. Anything else takes the existing `has-session` path: an existing session is `Ok` and unchanged, a missing one `usage`. (b) `run` first reads the directory with `list-panes -t =NAME: -F '#{session_path}'`, Decision 10's form, which fails closed (PR7). It refuses with `unavailable` unless the first line is an absolute path to an existing directory. The message names the rule and never the path. This read replaces `run`'s `has-session`. Do not use `display-message` for it: with a missing target it exits 0 and prints nothing (PR6). See Notes for O, items 1 to 7. |
| W-14 | warn | Decision 4 ("the pane child is a session and group leader ... so its children go too"; KILL "to every survivor", meaning a pane still listed) | completeness of the stop (the issue: stop "the processes that pane owns") | TERM reaches the whole group, but the escalation follows the pane. Once the leader exits, its pane is gone, and no KILL is ever sent. So a group member that ignores TERM outlives `stop_owned`, which returns `Ok`. Probed (PR2): the leader died on TERM and its pane left the listing, yet `kill -s 0 -- -L` still succeeded. The member, still in group L and reparented, ran until its own `sleep` ended. Such a survivor can hold the harness port, so the next `run` fails to serve. A process that left the group (through `setsid`, or by daemonizing) is never reached at all. | Preferred: after the shared grace, also send `kill -s KILL -- -<pgid>` to each TERMed group whose pane is gone but which `kill -s 0 -- -<pgid>` still finds, then poll until each such group is empty or the deadline passes. "No such process" still counts as gone. In AC 6h, add a listing that lacks `101` after TERM while the fake `kill` answers `-s 0 -- -101` with exit 0: the record gains `-s KILL -- -101`. W-8's bullet then reads "gone once the pane is dead or gone and `kill -s 0` reports No such process". At minimum, record both limits in Decision 15. |
| W-15 | warn | Decision 13 ("the pid as `u32`"); Decision 3's cleanup `kill`; Decision 4's listing parse | identity: never signal a stranger (the issue: "never by a broad pattern") | A `u32` admits values that the `kill` binary turns into a broad target or a wrong one. Probed with signal 0 on procps-ng 4.0.4 (PR3): `kill -s 0 -- -4294967295` (`u32::MAX`) reached pid 1 ("Operation not permitted"), because the argument wraps through `pid_t`. Every `u32` above `i32::MAX` wraps the same way to a positive pid, so a group kill becomes a kill of one unrelated process. In `kill(2)`, `-0` means the caller's own group and `-1` means every process the caller may signal. procps-ng happened to refuse `-0` and `-1` with a usage error, but util-linux `kill(1)` documents `kill -9 -1` as killing every process you can. A real tmux never prints such a pid, so this is defense in depth. It is also the one rule that keeps a `kill -- -1` out of the adapter by construction. | Accept a pid read from tmux only in `2..=i32::MAX`, both from `new-window -P` and from every listing. A pane whose `pane_pid` is out of range is never owned. Extend AC 6i's malformed answers with `0 @7`, `1 @7` and `4294967295 @7`: each is `unavailable`, with no `set-option` and no signal. AC 6h gains a live, tagged pane `4294967295 0 4294967295` that is never signalled. |
| W-16 | warn | AC 6e ("No recorded call has the bare name") | test-spec precision | `ensure_session` is in AC 6e's scope, and its correct vector is `new-session -d -s demo-c1r1`, whose `-s` value is a name, not a target. Read literally, AC 6e forbids that vector. The literal fix, `-s =demo-c1r1`, makes tmux create a session named `=demo-c1r1` (PR4). Every fake-tmux test would accept that, and only the ignored real-tmux tests would fail. | Reword: "No `-t` argument is the bare name. `new-session` records `-s demo-c1r1`, a name and not a target (`-s =NAME` would create a session literally named `=NAME`)." |
| W-17 | warn | Size estimate's fallback file `tests/support/fake.rs` | naming and file structure | The library-crate neighbors keep shared test helpers in `tests/common/mod.rs` (holler-adapter-herdr, holler-pane, holler-proto). `tests/support/` is holler-cli's process harness. A `tests/support/fake.rs` with no `mod.rs` also needs a `#[path]` attribute to be found. | If the split is needed, use `tests/common/mod.rs` with `#![allow(dead_code)] // #641`, as `holler-adapter-herdr/tests/common/mod.rs` does. |

Verified and consistent:

- **Round 2 is applied in full.**
  - B-4 is in Decisions 4, 5 and 10, AC 6e, AC 12, Risks and Evidence.
  - B-5 is in Decision 3, AC 6d, AC 11, AC 13, Decision 15 and the #644 row.
  - W-10 to W-13 land where the brief's header says.
  - O's re-run transcript in Evidence agrees with my round-2 probes.
- **No new duplication.** Remote `main` is still 3bdd129, so nothing has merged since round 2. No production crate runs a subprocess under a deadline. `holler-load-test` is a binary harness that calls `Command::output()` with no bound. So `exec.rs` is still the justified new object, with W-7's follow-up for #663.
- **Dependency direction.** It matches ADR-0021 §5 and `holler-adapter-herdr`: a normal dependency on `holler-pane` only, with the test kit as a dev-dependency. Only the CLI constructs an adapter (ruling 1, #649).
- **Crate root API.** `TmuxHost` and `TmuxSocket` sit at the crate root, while `holler-adapter-herdr` reaches every item by module path. Herdr's stated reason is root names that clash with `holler_proto`'s re-exports, and that does not arise here. Not drift.
- **B-6's read needs no new error case.** It answers through Decision 8's table unchanged: `can't find session` for a prefix or a window-name shadow, and `error connecting to ... (No such file or directory)` with no server (PR7). It also inherits Decision 10's colon form, which the brief already extends to "any command added later".
- **Hygiene.** AC 8's `rustfmt --edition 2021` matches the workspace edition. No tracked file holds the host name from the issue title. The outside-model prompt and result files are git-ignored.

## Probe evidence (round 3)

All probes ran on tmux 3.7c and procps-ng 4.0.4 (Linux). Each script ran its own private server: `tmux -S ../s -f /dev/null`, a relative socket path inside a fresh directory in the reviewer's session scratchpad, with `TMUX` and `TMUX_PANE` unset on every call. This session runs inside the operator's tmux, so the unset matters. An exit trap killed each server and removed its directory. A process check afterwards found no probe server and no probe directory left.

The only real signal sent was one TERM to the process group of a pane the probe itself started on its private server (PR2). Every other `kill` call used signal 0, which delivers nothing. The scripts (`a641r3-probe.sh`, `-probe2.sh`, `-probe3.sh`) are in the scratchpad, not in the repo. Below, `<dir>` is the probe directory, `<home>` the user's home directory, and L and M are the leader and member pids.

- **PR1, relative cwd.** From client cwd `<dir>/C`, `new-session -d -s demo-c1r1 -c rel` exited 0.
  - `display-message -p '#{session_path}'` printed `rel`, and the session's shell pane worked in `<dir>/C/rel`.
  - `new-window -d -P -F '#{pane_pid}' -c '#{session_path}' -t =demo-c1r1: -- env -- sleep 30`, from client cwd `<dir>/B` (where `B/rel` exists), started `sleep` in `<dir>/B/rel`.
  - The same command from `<dir>/E`, which has no `rel`, started it in `<home>`.
  - With `-c .`, `session_path` was `.`, and a run from `<dir>/B` worked in `<dir>/B`.
- **PR5, removed directory.** `new-session -c <dir>/W`, then `rmdir <dir>/W`. `session_path` was still `<dir>/W`, `new-window ... -c '#{session_path}' ...` exited 0, and the program worked in `<home>`.
- **PR6, `display-message` is not an existence check.** `display-message -p -t =NAME: '#{session_path}'` exited 0 with empty output in three cases: for `=nope:`, for `=demo-c1r1:` when only `demo-c1r10` existed, and for `=demo-c3r1:` when that was only a window name in another session. With no server it exited 1 (`no server running on ../s`).
- **PR7, `list-panes` as the read.** `list-panes -t =NAME: -F '#{session_path}'`, without `-s`:
  - `=demo-c1r10:` gave `<dir>/A` and `=demo-c2r1:` gave `rel` (a session made with `-c rel`), both exit 0.
  - `=demo-c1r1:` (a prefix of `demo-c1r10`) and `=demo-c3r1:` (only a window name) each gave `can't find session: ...`, exit 1.
  - With no server ever started it gave `error connecting to ../s (No such file or directory)`, exit 1.
- **PR2, a group member outlives its leader.** `run`'s vector started `env -- sh leader.sh` (leader L), which ran `sh member.sh &` and then waited. `member.sh` ran `trap '' TERM HUP` and then `exec sleep 6`. The window was tagged per Decision 3 (`tag exit 0`).
  - Member M showed pgid L and sid L.
  - The listing before TERM showed `[L 0 L]` and the shell pane.
  - `LC_ALL=C kill -s TERM -- -L` exited 0. 0.5 s later the listing held only the shell pane, while `kill -s 0 -- -L` still exited 0.
  - M was alive in group L, reparented, and exited on its own about 5 s later, with no further signal sent.
- **PR3, how `kill` reads a group argument (signal 0).**
  - `-0` and `-1`: a usage error, exit 1.
  - `-4294967295`: `Operation not permitted`, exit 1, meaning it reached pid 1.
  - `-4294967296`: exit 0, meaning it wrapped to the caller's own group.
  - `-4000000`: `No such process`, exit 1.
- **PR4, `new-session -s` takes a name.** `new-session -d -s =demo-c3r1` exited 0, and `list-sessions` listed a session named `=demo-c3r1`.

## Notes for O

These amendments are all additions, and no decision is reversed. A BLOCK stops this automated run. Recovery is to amend the brief and start a fresh run, not `resumeFromRunId`. This is the third Phase 3 BLOCK, so the role doc's escalation to the operator applies.

1. **Decision 6 (B-6a).** Replace "With a `cwd` that is an existing directory (checked on the unescaped value)" with "With a `cwd` that is an absolute path to an existing directory (both checked on the unescaped value)". Replace "With a `cwd` that is not an existing directory" with "With any other `cwd` (relative, or not an existing directory)". Add the reason: tmux keeps a relative `-c` verbatim and resolves it again against every later client's cwd (PR1).
2. **Decisions 3 and 7 (B-6b).** `run` first reads `list-panes -t =NAME: -F '#{session_path}'`. Then, in order:
   - a missing session (Decision 8's strings) is `pane-not-found`;
   - an empty argv is `usage`;
   - an `argv[0]` containing `=` is `usage`;
   - a first line that is not an absolute path to an existing directory is `unavailable`, with a fixed `what` such as "the session's directory is missing or not absolute", never the path (W-2);
   - otherwise `new-window ... -c '#{session_path}' ...` and the tag run, unchanged.

   This read replaces `run`'s `has-session` (Decision 7), so a successful `run` spawns **three** tmux subprocesses: the read, `new-window` and the tag. Add to Decision 10: "never `display-message` for an existence check: its target may fail silently (PR6)". Add PR1 and PR5 to PR7 to Evidence.
3. **Decision 12.** Add two narrowings: a relative `cwd` is `usage` when the session would be created, where the fake would create it; and `run` is `unavailable` when the session's directory is missing or not absolute, where the fake would run it. Put both in the crate docs' error table.
4. **Decision 15 and the #644 row.** Replace "so a later `run` still works in A" with "so a later `run` works in A while A exists. Once A is removed, `run` is `unavailable` until the session is ended, and no port call ends one (#644, #646)." In the #644 row add: a relative `host.cwd` is `usage` when the session would be created, so #644, or #670's flag layer, makes `--project` absolute before it is recorded.
5. **ACs.**
   - **AC 6d:** the calls file holds **three** entries for a successful `run`: the read (`list-panes -t =demo-c1r1: -F #{session_path}`, which the fake answers with an existing absolute tempdir), the `new-window` entry, then the tag. The rest of 6d stands.
   - **AC 6e:** the read records `-t =demo-c1r1:`, and W-16's rewording applies.
   - **AC 6i:** the fake's answer queue starts with the read's answer.
   - **AC 6g:** in its `argv[0]` bullet, "holds `has-session`" becomes "holds the read". Add two bullets:
     - `ensure_session` with the relative cwd `.` asks `has-session -t =demo-c1r1` and nothing else (`usage` / `Ok`, as for a non-directory), and never `new-session`.
     - `run` whose read answers `rel`, or an absolute path that does not exist, is `unavailable`. The calls file holds no `new-window`, and `what` holds neither path.
6. **Add AC 14, real tmux (`#[ignore]`), `run_refuses_a_missing_or_relative_directory` (B-6), built through the AC 9 helper:**
   - `ensure_session(demo-c1r1, <dir>/W)`; remove `<dir>/W`; then `run(demo-c1r1, ["sleep", "30"])` is `unavailable`, and `list-panes -s -t =demo-c1r1:` still lists only the shell pane.
   - `ensure_session(demo-c2r1, ".")` is `usage`, and `list-sessions` does not list `demo-c2r1`.
7. **Risks.** Extend the "Working directory" line: a relative or removed directory would also start the harness elsewhere (PR1, PR5). AC 6g and AC 14 pin both refusals.
8. **Warns.**
   - W-14: one Decision 4 sentence plus one AC 6h line, or the two limits in Decision 15 at minimum.
   - W-15: Decision 13's "the pid as `u32`" becomes "the pid as a number in `2..=i32::MAX`", applied to the listings too; AC 6i and AC 6h gain the cases above.
   - W-16: AC 6e's rewording.
   - W-17: the fallback path becomes `tests/common/mod.rs`.

**Size:** B-6 adds about 15 lines of production code (the absolute check, the read, the refusal, the docs) and about 60 lines of tests (AC 6d, 6e, 6g and 6i adjustments and AC 14). W-14 and W-15 add about 10 more lines of each. `fake_tmux_test.rs` reaches about 560 lines and `real_tmux_test.rs` about 415, both under the 800 flag. The work is still one run.

## Patterns referenced

- `crates/holler-pane/src/ports.rs:156-168` (the port: the session works "in `cwd`"); `crates/holler-cli/src/pane/args.rs:42-44` and `crates/holler-pane/src/pane.rs:105-111` (`--project` and `host.cwd` are plain strings, never canonicalized)
- `docs/adr/ADR-0021.md:136, 171-183` (`--project DIR` maps to `host.cwd`; verbs and adapters run CLI-side, and each adapter depends on `holler-pane` only)
- `crates/holler-pane/src/argv.rs:1-12` and `crates/holler-pane/src/error.rs:404-470` (guards that refuse bad input at the boundary and never echo it; the closed `usage` and `unavailable` variants the fixes use)
- `crates/holler-pane-testkit/src/host.rs:28-61, 208-214` (the `HostOp` names; the fake's rules and its missing-session-first order)
- `crates/holler-adapter-herdr/Cargo.toml`, `src/lib.rs`, `tests/common/mod.rs` (the neighbor adapter: dependencies, crate docs, shared test helpers)
