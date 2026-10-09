# Brief: #641 host-adapter (tmux sessions, process control, the launcher primitive)

Repo: Performant-Labs/holler. Issue: #641 (epic #633, wave 3). Rigor: in-session. UI surface: no. Kind: feature.

**Branch:** `issue-641-implementation` (worktree `.claude/worktrees/0641-host-adapter`, from `origin/main` at `9d61c9f`).
**Review-rigor:** in-session (the issue and the epic fix it; the change is one new adapter crate behind a frozen trait and a
conformance suite that already exists).
**Forward-compat:** done, see the table under "Decisions". **Design:** N/A (no UI surface).
**Decision record:** ADR-0021 (`docs/adr/ADR-0021.md`) section 2 and the epic's contract. This brief does **not** edit the ADR:
the design fits the frozen `HostPort` trait, and the one narrowing it makes (which processes are "owned") is recorded in the
adapter's rustdoc and below.
**Handoffs:** `docs/handoffs/641/handoff-<phase>.md` (T-red, F, T-green, A, A-dup, S); the decision journal is
`docs/handoffs/641/decisions.md`.

## Problem

`HostPort` (ensure a tmux session, run an argv in it, stop what it owns, list its pids) has a fake and a conformance suite
(#684) but no real implementation: `holler-adapter-host` is an empty skeleton. Without it no `holler pane` verb can create
the tmux session of record or start a harness in it, and launches stay keystroke-and-sleep (type a command into a shell, wait,
hope). This story builds `TmuxHost`, the real `HostPort`, which talks to a local tmux server (epic decision 3: no ssh), never
types into a shell, bounds every call, and stops only the processes it started.

There is **no launcher code in this repository to extend**: the keystroke-and-sleep launcher is the operator's external
tooling (pfleet), and the `launch` verb that composes these primitives is #644. In this repo `tmux` appears only in the
pane-control contract, the test kit and the spike scripts (`scripts/spikes/*.sh`, which drive Herdr, not tmux, with
`send-keys`). "The launcher" in the issue title is this adapter's `run`: it starts an argv in the session and returns once
tmux reports the new process, with no typing and no sleep.

## Evidence (verbatim, as of `9d61c9f`)

The port (frozen by #637):
```
crates/holler-pane/src/ports.rs:150-168
/// The machine a pane lives on: tmux sessions and the processes in them (the adapter
/// is `holler-adapter-host`, #641).
///
/// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
/// thread) in async code. Every method returns within I5's bound (default 10 s) or
/// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
pub trait HostPort: Send + Sync {
    /// Make the tmux session `name` exist, working in `cwd`.
    fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;

    /// Run `argv` (never through a shell) in the session `name`.
    fn run(&self, name: &PaneName, argv: &Argv) -> Result<(), PaneError>;

    /// Stop the processes the session `name` owns.
    fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;

    /// The process ids running in the session `name`.
    fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>;
}
```

The conformance suite the adapter must pass, and the readings it fixes (#684):
```
crates/holler-pane-testkit/src/conformance/host.rs:4-20
//! The cases pin what the tmux adapter (#641) must keep: `run` starts a process that
//! `ps` then lists, an empty argv is refused, `ensure_session` is idempotent, and
//! `stop_owned` stops every process of its session and none of another session's (no
//! broad kill). Two readings of the port are decided here (#684), and they bind the
//! adapter:
//!
//! - A missing session is `pane-not-found` for `run` and `ps`, which is how this suite
//!   reads #641's "a missing session is a typed error". The tmux session is named by the
//!   pane's name, so the pane's code fits; `session-not-found` stays the code of a
//!   harness session. An empty argv is `usage`.
//! - `stop_owned` of a missing session is `Ok`: nothing is owned, so nothing is
//!   stopped, and a `relaunch` or `close` after a crash does not fail on it (case 7).
//!
//! The cases assert only what a real tmux session can also satisfy. A real session may
//! hold a shell process, so no case asserts that a fresh session's `ps` is empty; and it
//! may end with its last process, so a case ensures the session again before it reads
//! `ps` after `stop_owned`. Every list of pids is read as a set: the port fixes no order.
```
```
crates/holler-pane-testkit/src/conformance/host.rs:75-93
/// Run every case of [`host_cases`], in order, each against a fresh host with no
/// session, and return every case that did not hold.
///
/// `fresh` is called once per case. It returns the host and a guard that the suite
/// keeps alive for that case only; the host is dropped before its guard. How each
/// implementation runs the suite (the test kit cannot name an adapter):
///
/// ```text
/// // the fake:
/// assert_eq!(run_host_conformance(|| (FakeHost::new(), ())), Ok(()));
/// // holler-adapter-host (#641): a private tmux server per case
/// //   (`tmux -S <tempdir>/tmux.sock`), the adapter on it as the host, and the tempdir
/// //   and the tmux server's handle as the guard, so dropping the guard kills every
/// //   process the case started.
/// ```
pub fn run_host_conformance<S, K, F>(fresh: F) -> Conformance
where
    S: HostPort,
    F: FnMut() -> (S, K),
```
The nine case ids (`host_cases()`), every one run with the argv `["sleep", "30"]`, sessions `demo-c1r1`/`demo-c2r1`, cwd
`std::env::temp_dir()`: `ps-of-missing-session-is-pane-not-found`, `run-in-missing-session-is-pane-not-found`,
`run-adds-a-process` (the pid `run` added must appear in the very next `ps`), `ensure-session-is-idempotent`,
`run-empty-argv-is-usage`, `stop-owned-stops-every-owned-process` (re-ensures the session, then `ps` must lack both pids),
`stop-owned-of-missing-session-is-ok`, `stop-owned-leaves-other-sessions`, `ps-lists-only-its-session`.

The fake's rules, which the adapter must match on every input (the verbs are tested on the fake):
```
crates/holler-pane-testkit/src/host.rs:52-61
/// - `ensure_session` creates a missing session working in `cwd`; on an existing one it
///   is `Ok` and changes nothing, neither the cwd nor the processes.
/// - `run` in a missing session is `pane-not-found`, and an empty argv is `usage`.
///   Otherwise it starts one process in the session and records the argv exactly as
///   given: an element is never joined to another or re-split, and nothing goes
///   through a shell.
/// - `stop_owned` stops every process of the session and of no other session. On a
///   missing session it is `Ok`: nothing is owned, so nothing is stopped.
/// - `ps` of a missing session is `pane-not-found`; otherwise the session's pids, in
///   the order they started.
crates/holler-pane-testkit/src/host.rs:208-214   (order: the missing session is checked BEFORE the empty argv)
    fn start(&mut self, name: &PaneName, argv: &Argv) -> Result<(), PaneError> {
        let session = self.sessions.get_mut(name).ok_or_else(|| not_found(name))?;
        if argv.as_slice().is_empty() {
            return Err(PaneError::Usage {
                message: "an empty argv has no program to run".to_owned(),
            });
        }
```

The error variants used (closed set, not edited here):
```
crates/holler-pane/src/error.rs:409, 456, 458, 467
    Usage { message: String },
    Timeout { op: String },
    PaneNotFound { what: String },
    Unavailable { what: String },
```

The crate today (both files replaced/extended by this story):
```
crates/holler-adapter-host/Cargo.toml:12-15
# Declare only what is consumed (issue #155 §7 — `cargo machete` fails CI
# otherwise). The crate is an empty skeleton; its owning story adds the
# dependencies it uses.
[dependencies]
crates/holler-adapter-host/src/lib.rs:1-5
//! `holler_adapter_host` — the host adapter: it implements `holler_pane::HostPort`
//! (tmux sessions and the processes in them) for the machine the hub runs on
//! (epic #633).
//!
//! Empty skeleton (story #637); story #641 fills it.
```

The consumer seam (owned by #649, **not touched here**): `crates/holler-cli/src/pane/wiring.rs:25-34` builds `Wiring` with
the `Unwired` stub whose `impl HostPort for Unwired` (line 151) answers `not-implemented`. #649 will replace it with this
crate's `TmuxHost`.

Workspace rules that bind the code: `Cargo.toml` `[workspace.lints]` denies `unwrap_used`, `expect_used`, `panic`,
`unreachable`, `cognitive_complexity` (threshold 15), `too_many_lines` (threshold 100 per fn) and `dead_code`; test files open
with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` (as `holler-pane-testkit/tests/*.rs` do). `libc` is
in the workspace only for test harnesses that use `unsafe`; the epic forbids new `unsafe`. `tempfile = "3"` is a workspace dep.

tmux behaviour this design relies on, observed by O on tmux 3.7c against a private `-L` socket (never the default socket):
```
has-session -t demo            (session "demo-c1r1" exists)  -> exit 0      PREFIX MATCH: the bare target is unsafe
has-session -t =demo                                         -> "can't find session: demo", exit 1
list-panes -s -t =nope ...     (server up, no such session)  -> "can't find window: nope", exit 1
new-window -t =nope: -- ...                                  -> "can't find session: nope", exit 1
has-session / list-panes, no server ever started             -> "error connecting to <sock> (No such file or directory)", exit 1
list-panes after kill-server (stale socket file)             -> "no server running on <sock>", exit 1
socket path longer than sun_path                             -> "error connecting to <sock> (File name too long)", exit 1
new-session -d -s demo-c1r1 (exists)                         -> "duplicate session: demo-c1r1", exit 1, nothing changed
new-window -d -P -F '#{pane_pid} #{window_id}' -t =demo-c1r1: -- sleep 30
                                                             -> "1505489 @1"; ps: PID=PGID=SID=1505489, comm sleep (no shell)
new-window ... -- 'sleep 31'   (ONE argument)                -> runs "sh -c sleep 31": tmux hands a single argument to a shell
new-window ... -- env -- sleep 32                            -> pane_pid's comm is sleep, PGID == pid (env execs in place)
new-window ... -- env -- 'sleep 33'                          -> exits at once (no program "sleep 33"): no shell involved
set-option -w -t @1 @holler-pid 1505489; list-panes -F '#{pane_pid} #{@holler-pid}' -> "1505489 1505489"
kill -s TERM -- -1505489                                     -> the window closes; list-panes no longer lists it
kill-server                                                  -> leaves the socket FILE behind (a guard must delete its dir)
```

## Acceptance criteria

All run from the worktree. "Real-tmux" tests are `#[ignore]` (the issue's opt-in) and are run explicitly; each one returns
early with an `eprintln!("skipped: tmux not found")` when `tmux -V` cannot run, so `-- --ignored` on a machine without tmux
passes and says why.

1. **Conformance (real tmux).** `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored
   the_tmux_host_passes_the_host_conformance_suite` passes: `run_host_conformance(fresh)` returns `Ok(())`, where `fresh`
   builds a `TmuxHost` on a private server per case (AC 9) and returns the guard. Every one of the nine cases holds, which
   fixes: `ps`/`run` on a missing session is `pane-not-found`; `stop_owned` on a missing session is `Ok`; `run` of an empty
   argv in an existing session is `usage` and starts nothing; `run` in a missing session with an empty argv is
   `pane-not-found` (the fake's order).
2. **Ensure twice makes one session (real tmux, ignored):** `ensure_session_twice_makes_one_session` calls `ensure_session`
   twice (the second with a different cwd) and asserts `tmux -S <sock> list-sessions -F '#{session_name}'` prints exactly one
   line, `demo-c1r1`, and the session's `#{session_path}` is still the first cwd.
3. **Stop kills only the owned process (real tmux, ignored):** `stop_owned_kills_only_the_owned_process` runs `sleep 30` in
   `demo-c1r1` and in `demo-c2r1`, and the test itself spawns a third `sleep 30` outside tmux (`std::process::Command`, killed
   by the test at the end). After `stop_owned(demo-c1r1)`: the c1r1 run's pid is gone from `ps` and from the OS (`kill -0`
   fails, checked with the `kill` binary); the session `demo-c1r1` still exists and its shell pane (the pane `ensure_session`
   created) is still in `ps`; the c2r1 pid and the outside `sleep` (same program name) are alive.
4. **Escalation (real tmux, ignored):** `stop_owned_escalates_to_kill` runs `["sh", "-c", "trap '' TERM HUP; sleep 30"]`, sets
   the stop grace to 200 ms, and asserts `stop_owned` returns `Ok` within the bound and the pid is gone.
5. **Missing session is a typed error (real tmux, ignored):** `missing_session_is_typed` on a private socket where **no
   server was ever started**: `ps` and `run` are `PaneError::PaneNotFound`, `stop_owned` is `Ok` (proves the "error
   connecting ... No such file or directory" path maps like "can't find session").
6. **No tmux needed by default.** `cargo test -p holler-adapter-host` (no `--ignored`) passes on a machine without tmux. Its
   tests drive `TmuxHost` against a **fake tmux binary**: a `/bin/sh` script the test writes into a tempdir, which appends its
   arguments to a calls file and answers with a canned stdout, stderr and exit status. They assert:
   - a. **Bounded:** with a fake that runs `exec sleep 30`, each of the four methods returns `PaneError::Timeout { op }` with
     `op` equal to `host.ensure_session` / `host.run` / `host.stop_owned` / `host.ps` (the `HostOp::as_str` names of the
     test kit) within the configured timeout (200 ms) plus 1 s, and the hung fake process is reaped (its pid, written by the
     script to a file, no longer exists).
   - b. **Missing binary:** a tmux path that does not exist gives `PaneError::Unavailable` from all four methods (including
     `stop_owned`: a missing binary is not a missing session).
   - c. **Error mapping:** stderr `can't find session: x`, `can't find window: x`, `no server running on /s`, `error connecting
     to /s (No such file or directory)` (exit 1) each map to `PaneNotFound` for `ps` and `run` and to `Ok` for `stop_owned`;
     stderr `error connecting to /s (Permission denied)` or `(File name too long)` and any other non-zero exit map to
     `Unavailable` whose `what` carries tmux's first stderr line.
   - d. **Argv passed exactly, never through a shell:** `run(demo-c1r1, ["prog", "a b", "$(id);x", "-t"])` makes the fake
     record, as separate arguments and in this order, `-S <sock>` first, then `new-window -d -P -F #{pane_pid} #{window_id}
     -t =demo-c1r1: -- env -- prog "a b" "$(id);x" -t`; a one-element argv `["prog"]` is also prefixed with `env --` (so tmux
     never sees a single argument, which it would give to a shell). The record shows a `set-option -w -t <window_id>
     @holler-pid <pid>` call after it, with the pid and window id the fake printed.
   - e. **Exact targets:** every recorded call that names the session uses `=demo-c1r1` (or `=demo-c1r1:`), never the bare
     name (tmux prefix-matches a bare target).
   - f. **Socket flags:** `TmuxSocket::Path(p)` puts `-S p` and `TmuxSocket::Name(n)` puts `-L n` before the subcommand;
     `TmuxSocket::Default` puts neither; a configured config file adds `-f <path>`. Every spawned tmux has `TMUX` and
     `TMUX_PANE` removed from its environment: the test re-runs its own test binary (`std::env::current_exe()`) as a child
     with `TMUX=/nonexistent,1,0` and `TMUX_PANE=%99` set on that child `Command` and a filter naming one helper test (which
     returns at once unless a marker variable is set); the helper drives the adapter against the fake, the fake writes
     `${TMUX-unset} ${TMUX_PANE-unset}` to a file, and the parent asserts the file reads `unset unset`. No `set_var`, no
     `unsafe`.
   - g. **Bad input refused before tmux:** `ensure_session` with a `cwd` that is not an existing directory is `usage` and the
     calls file is empty; `run` whose `argv[0]` contains `=` is `usage` (env would read it as an assignment).
7. **No broad kill:** `grep -rnE 'pkill|killall|pgrep|pidof' crates/holler-adapter-host/src` prints nothing, and a default-run
   test `no_broad_kill_in_source` asserts the same over `include_str!` of every `src/*.rs`. Signals go only to a process group
   whose leader pid tmux reports for a window the adapter tagged (Decisions 3, 4).
8. **Quality gates:** `cargo build --workspace`, `cargo clippy -p holler-adapter-host --all-targets -- -D warnings`, `rustfmt
   --check --edition 2021` on every new `.rs` file, and `cargo test --workspace` all pass; `cargo machete` (CI) finds no unused
   dependency; no `unsafe` in the crate (`grep -rn unsafe crates/holler-adapter-host` prints nothing); every touched file is
   under 900 lines and every function under clippy's 100-line threshold.
9. **Isolation of real-tmux tests:** every real-tmux test builds its host only through one test helper that creates
   `tempfile::Builder::new().prefix("hlr-tmux-").tempdir_in("/tmp")`, uses `TmuxSocket::Path(<dir>/s)` (asserting the path is
   under 100 bytes, the `sun_path` limit), and a config file in that dir containing `set -g default-shell /bin/sh` (no user
   rc). Its guard's `Drop` runs `tmux -S <dir>/s kill-server` (ignoring "no server") and then removes the dir.
   `grep -rn 'TmuxSocket::Default\|TmuxSocket::Name' crates/holler-adapter-host/tests` prints nothing. No test names a session
   outside `demo-*`.
10. **CHANGELOG:** `CHANGELOG.md` `## [Unreleased]` / `### Enhancements` gains one entry for the host adapter linking
    `[#641](https://github.com/Performant-Labs/holler/issues/641)`.

## Files

Production (crate `crates/holler-adapter-host/`):
- `Cargo.toml` (edit): `[dependencies] holler-pane = { path = "../holler-pane" }`; `[dev-dependencies] holler-pane-testkit
  = { path = "../holler-pane-testkit" }`, `tempfile = { workspace = true }`; each with the `# for ...` consumer comment the
  workspace uses. Update `description` (drop "empty in the workspace skeleton").
- `src/lib.rs` (rewrite, ~250 lines): crate docs (the error table and the ownership rule), `TmuxSocket`, `TmuxHost` with its
  builder, `impl HostPort for TmuxHost`.
- `src/exec.rs` (new, ~150 lines): run one child process with a deadline (stdin null, stdout and stderr drained on threads,
  `try_wait` polling, `Child::kill` + `wait` on the deadline), returning status/stdout/stderr or `Timeout`; spawn `NotFound`
  maps to `Unavailable`. Also the `kill -s <SIG> -- -<pgid>` call.
- `src/tmux.rs` (new, ~180 lines): the argument lists for each tmux call (pure functions), parsing `list-panes` /
  `new-window -P` output, and classifying a failed call's stderr into Missing / Other.

Tests:
- `tests/fake_tmux_test.rs` (new, ~350 lines): AC 6 and AC 7, default run.
- `tests/real_tmux_test.rs` (new, ~250 lines): AC 1-5 and the AC 9 helper, all `#[ignore]`.

Outside the crate (mechanical): `CHANGELOG.md` (AC 10) and `Cargo.lock` (the new path deps).

**Size estimate:** ~580 production lines, ~600 test lines, ~1,200 in all; six crate files plus two mechanical ones. One
component family (one crate behind one trait): **fits one run**, no split.

**Blast radius:** `crates/holler-adapter-host/**` (the issue's), plus `CHANGELOG.md` and `Cargo.lock`. No other crate
changes; nothing depends on `holler-adapter-host` yet, so no existing test can break.

**Reuse map (extend, do not duplicate):**

| Object | Use | Extend or new |
|---|---|---|
| `holler_pane::HostPort` (`ports.rs:156`) | implemented as is; signature frozen | extend (implement) |
| `holler_pane_testkit::conformance::host::run_host_conformance` | the acceptance test, with a real-tmux `fresh` and guard | reuse, no copy of its cases |
| `holler_pane_testkit::host::HostOp::as_str` names (`host.run`, ...) | the `op` of every `Timeout` | reuse the strings (the adapter does not depend on the test kit at runtime; it uses the same literals, and AC 6a pins them equal by comparing with `HostOp::as_str` in the test) |
| `holler_pane::{PaneError, PaneName, Argv}` | errors, names, argv | reuse; no new error variant |
| `holler-adapter-host` skeleton | filled | extend |
| `holler-cli/src/pane/wiring.rs` | **not touched** (#649 owns it) | n/a |
| `run_probe` (`probe.rs`), a stub until #663 | not reused: its body is #663's, and its contract (a `ProbeResult`, no `PaneError`) does not fit; `exec.rs` stays private to this crate | new, justified |
| existing launcher | none in this repo (see Problem) | n/a |

## Decisions already made (O)

1. **Type and constructor.** `pub struct TmuxHost` (plain data, so `Send + Sync`), `pub enum TmuxSocket { Default,
   Name(String), Path(PathBuf) }`, `TmuxHost::new(socket)`; builder methods `with_tmux_binary(PathBuf)` (default `tmux`
   from `PATH`), `with_config(PathBuf)` (adds `-f`), `with_timeout(Duration)` (default 10 s, I5), `with_stop_grace(Duration)`
   (default 2 s). #649 will call `TmuxHost::new(TmuxSocket::Default)`; #667 uses `Path`.
2. **One deadline per method.** Each port call computes one deadline (`now + timeout`) and every tmux or `kill` subprocess it
   spawns gets the time remaining; on expiry the child is killed and reaped and the method returns `Timeout { op:
   "host.<method>" }`. A hung call is reported, not waited on.
3. **Ownership = what `run` started, by recorded pid.** `run` executes `new-window -d -P -F '#{pane_pid} #{window_id}' -t
   =NAME: -- env -- <argv...>`, then `set-option -w -t <window_id> @holler-pid <pid>`. The record lives in tmux (not in a file,
   I6; the adapter is rebuilt per CLI run so it cannot keep memory). `stop_owned` stops exactly the panes of the session whose
   `#{@holler-pid}` equals their live `#{pane_pid}`; the shell window `ensure_session` created, and anything a person opened, is
   never signalled, so the session (and a Herdr pane attached to it) survives a stop. If the window is already gone when
   `set-option` runs (the program exited at once), `run` still returns `Ok`.
4. **How it stops.** For each owned pid: `kill -s TERM -- -<pid>` (the pane child is a session and group leader, PGID == pid,
   so its children go too); poll `list-panes` every 50 ms until the pane is gone or the grace ends; then `kill -s KILL --
   -<pid>`; poll until gone or the deadline (`Timeout`). `kill` reporting "No such process" counts as gone. The `kill` binary is
   used instead of `libc::kill` so the crate stays free of `unsafe` and of new dependencies. Nothing matches by name.
5. **`ps`** = `list-panes -s -t =NAME -F '#{pane_pid} #{pane_dead}'`: the leader pid of every live pane of the session
   (the shell included; descendants not listed), as the port fixes no order. Untagged panes count: `ps` reports what runs
   there; `stop_owned` decides what to stop.
6. **`ensure_session`** = validate `cwd` is an existing directory (else `usage`, no tmux call), then `new-session -d -s NAME -c
   CWD`; `duplicate session` is `Ok` and changes nothing (one call, race-free).
7. **`run` order** matches the fake: a missing session first (`pane-not-found`), then an empty argv (`usage`). With an empty
   argv the adapter asks `has-session -t =NAME` to tell the two apart; with a non-empty one `new-window` itself reports the
   missing session. `argv[0]` containing `=` is `usage` (env would treat it as an assignment).
8. **Error mapping** (the crate docs carry this table): stderr containing `can't find session`, `can't find window`, `no
   server running`, or `error connecting to` with `No such file or directory` or `Connection refused` means **missing**
   (`pane-not-found` for `run`/`ps`, `Ok` for `stop_owned`); a spawn `NotFound` or any other failure is `unavailable` with the
   first stderr line.
9. **Environment.** Every spawned `tmux` has `TMUX` and `TMUX_PANE` removed, so a hub or test running inside a tmux pane never
   addresses that pane's server by inheritance; only the configured socket is used (with `TmuxSocket::Default` that means
   tmux's own default socket, never `$TMUX`'s). AC 6f checks it with a re-executed child test, not `set_var`.
10. **Exact targets.** Every session target is `=NAME` or `=NAME:` (tmux prefix-matches a bare name: `has-session -t demo`
    succeeded for `demo-c1r1`). Pane names are `[a-z0-9-]` (ADR 0005), so no further escaping is needed.
11. **Tests, not CI.** Real-tmux tests are `#[ignore]` per the issue and are run by T-green and S with `-- --ignored` on a
    machine that has tmux (the pipeline host has 3.7c). Adding them to CI needs an edit to `.github/workflows/ci.yml`, outside
    this blast radius: a follow-up for the operator, not this story.
12. **ADR-0021 unchanged.** The ownership rule (3) narrows "stop the processes the session owns" without changing a type or
    method; it is recorded in the crate docs and the CHANGELOG.

Forward-compat (consumers of this crate):

| Consumer | Needs | Satisfied |
|---|---|---|
| #644 launch/relaunch | `ensure_session` then `run(command argv)`; relaunch = `stop_owned` then `run` without killing the session | yes (3, 6) |
| #646 close | `stop_owned` that is `Ok` on a crashed/missing session | yes (8) |
| #647 reconcile/doctor | `ps` as observed state, `pane-not-found` for a session gone | yes (5, 8) |
| #649 wiring | a constructor with no I/O, `Send + Sync`, the default socket | yes (1) |
| #667 apply scenario | the adapter on a scratch tmux server | yes (`TmuxSocket::Path`/`Name`) |
| #642 OpenCode adapter | none: `HarnessPort::serve` returns its own pid; the host adapter does not stop by port | n/a |

## Out of scope

- Wiring the adapter into the CLI (`wiring.rs`, #649); any verb (#644, #646, #647).
- Stopping by harness port (the issue's "pid and port"): `HostPort::stop_owned` takes only a name, and the server's port is
  `HarnessPort`'s (#642). Recorded pids are the only handle here.
- A tmux version check, remote hosts, editing ADR-0021 or the test kit, CI changes.
- Touching any live tmux server, the default socket, the operator's sessions, Herdr or OpenCode.

## Test plan

RED: T writes both test files against the API in Decisions 1 (it compiles once F's public types exist; until then RED is a
compile failure of the new tests, which T records with the exact `cargo test -p holler-adapter-host` error). If T prefers a
behavioural RED, F may first land the public types with every method returning `NotImplemented`; then every default-run test
fails on its assertion and every real-tmux test fails on `not-implemented`. T records which.
GREEN: `cargo test -p holler-adapter-host` (default) and `cargo test -p holler-adapter-host -- --ignored` (real tmux,
private sockets), then the AC 8 gates. T-green also confirms no `hlr-tmux-*` dir is left in `/tmp` and no `demo-*` session
exists on the default socket (`tmux list-sessions` on the default socket is read-only and must not list one; if no default
server runs, that is fine).

## Risks

- **tmux single-argument shell path.** tmux runs a one-argument command through `sh -c`; the `env --` prefix (AC 6d) is what
  keeps "never through a shell" true. A refactor that drops it reopens injection; AC 6d pins it.
- **Prefix matching** would let `stop_owned(demo)` hit `demo-c1r1`; AC 6e pins `=`.
- **Socket path length:** `sun_path` is ~104-108 bytes; a long `TMPDIR` gives "File name too long". The test helper uses
  `/tmp` and asserts the length (AC 9).
- **Pid reuse** between `list-panes` and `kill` is a window of milliseconds; tmux's live `pane_pid` plus the matching
  `@holler-pid` makes a stale target unlikely. Accepted.
- **`kill` binary portability** (`kill -s TERM -- -PGID`): verified on Linux procps; macOS BSD `kill` accepts the same form.
  Real-tmux tests are opt-in, so a macOS difference cannot break CI.
- **Shell rc in sessions:** production sessions use the operator's default shell; tests force `/bin/sh` through the config
  file so they do not depend on a user's rc files.
